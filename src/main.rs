use clap::Parser;
use std::io::IsTerminal;
use tonic::Request;

// Include the auto-generated gRPC code
// Nested module structure required for proper proto cross-references
pub mod api {
    pub mod observer {
        tonic::include_proto!("observer");
    }
    pub mod flow {
        tonic::include_proto!("flow");
    }
    pub mod relay {
        tonic::include_proto!("relay");
    }
}

mod formatter;
mod stats_formatter;
use formatter::FlowFormatter;
use stats_formatter::StatsFormatter;

use api::observer::observer_client::ObserverClient;
use api::observer::GetFlowsRequest;

fn is_internal_ip(ip: &str) -> bool {
    ip.starts_with("10.42.")
}

fn should_show_flow(world_only: bool, source_ip: Option<&str>, dest_ip: Option<&str>) -> bool {
    if !world_only {
        return true;
    }

    let source_ip = source_ip.unwrap_or("");
    let dest_ip = dest_ip.unwrap_or("");

    !is_internal_ip(source_ip) || !is_internal_ip(dest_ip)
}

fn should_show_flow_with_filters(
    world_only: bool,
    filter_ips: &[String],
    exclude_ips: &[String],
    source_ip: Option<&str>,
    dest_ip: Option<&str>,
) -> bool {
    // 1. First check world-only filter
    if world_only {
        if !should_show_flow(world_only, source_ip, dest_ip) {
            return false;
        }
    }

    // 2. Check exclude filter - REMOVE matching flows
    if !exclude_ips.is_empty() {
        let src_ip = source_ip.unwrap_or("");
        let dst_ip = dest_ip.unwrap_or("");

        let src_match = exclude_ips.iter().any(|ip| ip == src_ip);
        let dst_match = exclude_ips.iter().any(|ip| ip == dst_ip);

        // If either IP matches exclude list, SKIP this flow
        if src_match || dst_match {
            return false;
        }
    }

    // 3. Check filter-ip filter - KEEP only matching flows
    if !filter_ips.is_empty() {
        let src_ip = source_ip.unwrap_or("");
        let dst_ip = dest_ip.unwrap_or("");

        let src_match = filter_ips.iter().any(|ip| ip == src_ip);
        let dst_match = filter_ips.iter().any(|ip| ip == dst_ip);

        // If neither IP matches filter list, SKIP this flow
        if !src_match && !dst_match {
            return false;
        }
    }

    true
}

async fn handle_stats(args: &CliArgs, colored: bool) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tokio::signal;
    use tokio::time::{interval, Duration};

    let endpoint = format!("http://{}:{}", args.address, args.port);
    println!("Connecting to Hubble gRPC API at {}...", endpoint);
    let mut client = ObserverClient::connect(endpoint).await?;

    let request = Request::new(GetFlowsRequest {
        number: 0,
        follow: true,
        whitelist: vec![],
        blacklist: vec![],
        ..Default::default()
    });
    let mut stream = client.get_flows(request).await?.into_inner();
    println!("Collecting flow statistics (Press Ctrl+C to stop)...\n");

    let ip_counts: Arc<Mutex<HashMap<(String, String), u64>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let counts_clone = ip_counts.clone();

    let mut interval = interval(Duration::from_secs(2));
    let formatter = StatsFormatter::new(colored);
    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            _ = &mut ctrl_c => {
                return Ok(());
            }
            _ = interval.tick() => {
                print!("\r\x1b[2J\x1b[H");

                let counts = ip_counts.lock().unwrap();
                let mut stats: Vec<(String, String, u64)> = counts
                    .iter()
                    .map(|((src, dst), count)| (src.clone(), dst.clone(), *count))
                    .collect();

                stats.sort_by(|a, b| b.2.cmp(&a.2));

                println!("{}", formatter.format_stats(&stats));
            }
            msg = stream.message() => {
                match msg {
                    Ok(Some(response)) => {
                        if let Some(api::observer::get_flows_response::ResponseTypes::Flow(flow)) =
                            &response.response_types
                        {
                            if let Some(ip) = &flow.ip {
                                // Apply filters if any is enabled
                                if args.world_only
                                    || !args.filter_ip.is_empty()
                                    || !args.exclude.is_empty()
                                {
                                    if !should_show_flow_with_filters(
                                        args.world_only,
                                        &args.filter_ip,
                                        &args.exclude,
                                        Some(&ip.source),
                                        Some(&ip.destination),
                                    ) {
                                        continue;
                                    }
                                }

                                let mut counts = counts_clone.lock().unwrap();
                                let key = (ip.source.clone(), ip.destination.clone());
                                *counts.entry(key).or_insert(0) += 1;
                            }
                        }
                    }
                    Ok(None) | Err(_) => {
                        return Ok(());
                    }
                }
            }
        }
    }
}

#[derive(Parser)]
#[command(subcommand_required = true)]
#[command(author, version, about)]
struct CliArgs {
    #[arg(long, default_value = "127.0.0.1")]
    address: String,

    #[arg(long, default_value = "4245")]
    port: u16,

    #[arg(long)]
    world_only: bool,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Filter flows by IP addresses. Only flows where source or destination IP matches one of the specified IPs will be displayed. Multiple IPs can be specified using comma separation (e.g., --filter-ip 10.42.1.5,10.42.2.10)"
    )]
    filter_ip: Vec<String>,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Exclude flows by IP addresses. Flows where source or destination IP matches one of the specified IPs will be filtered out. Multiple IPs can be specified using comma separation (e.g., --exclude 10.42.1.5,10.42.2.10)"
    )]
    exclude: Vec<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
enum Commands {
    Flow,
    Stats,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let args = CliArgs::parse();

    // Dispatch to subcommand handler
    match args.command {
        Commands::Flow => {
            // Always output colored
            let colored = std::io::stdout().is_terminal();
            let formatter = FlowFormatter::new(colored);

            // 1. Connect to the Hubble gRPC Relay (ensure 'cilium hubble port-forward' is running)
            let endpoint = format!("http://{}:{}", args.address, args.port);
            println!("Connecting to Hubble gRPC API at {}...", endpoint);
            let mut client = ObserverClient::connect(endpoint).await?;

            // 2. Formulate the request. Follow = true means continuous stream.
            let request = Request::new(GetFlowsRequest {
                number: 0,
                follow: true,
                whitelist: vec![], // Add filters here if you only want DROPPED packets
                blacklist: vec![],
                ..Default::default()
            });

            // 3. Open the gRPC stream
            let mut stream = client.get_flows(request).await?.into_inner();
            println!("Listening for network events (Press Ctrl+C to stop)...\n");

            // 4. Iterate over the stream asynchronously
            while let Some(response) = stream.message().await? {
                // Apply filters if any is enabled
                if args.world_only || !args.filter_ip.is_empty() || !args.exclude.is_empty() {
                    if let Some(api::observer::get_flows_response::ResponseTypes::Flow(flow)) =
                        &response.response_types
                    {
                        if let Some(ip) = &flow.ip {
                            if !should_show_flow_with_filters(
                                args.world_only,
                                &args.filter_ip,
                                &args.exclude,
                                Some(&ip.source),
                                Some(&ip.destination),
                            ) {
                                continue;
                            }
                        }
                    }
                }

                let formatted = formatter.format_flow(&response);
                if !formatted.is_empty() {
                    println!("{}", formatted);
                    println!();
                }
            }
        }
        Commands::Stats => {
            let colored = std::io::stdout().is_terminal();
            handle_stats(&args, colored).await?;
        }
    }

    Ok(())
}
