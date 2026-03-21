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
use formatter::{FlowFormatter, Verbosity};

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

#[derive(Parser)]
#[command(author, version, about)]
struct CliArgs {
    #[arg(long, default_value = "127.0.0.1")]
    address: String,

    #[arg(long, default_value = "4245")]
    port: u16,

    #[arg(long, default_value = "minimal", value_parser = ["minimal", "normal", "verbose"])]
    verbosity: String,

    #[arg(long)]
    no_color: bool,

    #[arg(long)]
    world_only: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let args = CliArgs::parse();

    // Determine verbosity level
    let verbosity = match args.verbosity.as_str() {
        "minimal" => Verbosity::Minimal,
        "normal" => Verbosity::Normal,
        "verbose" => Verbosity::Verbose,
        _ => Verbosity::Minimal,
    };

    // Detect if stdout is a terminal for color support
    let colored = !args.no_color && std::io::stdout().is_terminal();

    // Create formatter
    let formatter = FlowFormatter::new(verbosity, colored);

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
        if args.world_only {
            if let Some(api::observer::get_flows_response::ResponseTypes::Flow(flow)) =
                &response.response_types
            {
                if let Some(ip) = &flow.ip {
                    if !should_show_flow(args.world_only, Some(&ip.source), Some(&ip.destination)) {
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

    Ok(())
}
