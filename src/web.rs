use actix::AsyncContext;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::Duration;
use tonic::Request;

use crate::api::observer::{observer_client::ObserverClient, GetFlowsRequest};
use crate::formatter::FlowFormatter;

/// JSON structure for WebSocket flow data
#[derive(serde::Serialize, Clone)]
pub struct FlowData {
    pub id: u64,
    pub timestamp: String,
    pub source: SourceInfo,
    pub destination: DestInfo,
    pub protocol: String,
    pub verdict: String,
}

#[derive(serde::Serialize, Clone)]
pub struct SourceInfo {
    pub namespace: String,
    pub pod: String,
    pub ip: String,
    pub port: u32,
}

#[derive(serde::Serialize, Clone)]
pub struct DestInfo {
    pub namespace: String,
    pub pod: String,
    pub ip: String,
    pub port: u32,
}

/// Parse a flow message and extract relevant fields
fn parse_flow_to_web_data(response: &crate::api::observer::GetFlowsResponse) -> Option<FlowData> {
    if let Some(crate::api::observer::get_flows_response::ResponseTypes::Flow(flow)) =
        &response.response_types
    {
        let timestamp = if let Some(time) = &flow.time {
            chrono::DateTime::from_timestamp(time.seconds, time.nanos as u32)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let mut source_ip = String::new();
        let mut dest_ip = String::new();
        let mut source_port = 0u32;
        let mut dest_port = 0u32;
        let mut source_ns = String::new();
        let mut source_pod = String::new();
        let mut dest_ns = String::new();
        let mut dest_pod = String::new();

        if let Some(ip) = &flow.ip {
            source_ip = ip.source.clone();
            dest_ip = ip.destination.clone();
        }

        if let Some(ip) = &flow.ip {
            if let Some(l4) = &flow.l4 {
                if let Some(proto) = &l4.protocol {
                    match proto {
                        crate::api::flow::layer4::Protocol::Tcp(tcp) => {
                            source_port = tcp.source_port;
                            dest_port = tcp.destination_port;
                        }
                        crate::api::flow::layer4::Protocol::Udp(udp) => {
                            source_port = udp.source_port;
                            dest_port = udp.destination_port;
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(source) = &flow.source {
            source_ns = source.namespace.clone();
            source_pod = source.pod_name.clone();
        }

        if let Some(dest) = &flow.destination {
            dest_ns = dest.namespace.clone();
            dest_pod = dest.pod_name.clone();
        }

        let verdict = match flow.verdict {
            x if x == crate::api::flow::Verdict::Forwarded as i32 => "FORWARDED".to_string(),
            x if x == crate::api::flow::Verdict::Dropped as i32 => "DROPPED".to_string(),
            x if x == crate::api::flow::Verdict::Audit as i32 => "AUDIT".to_string(),
            x if x == crate::api::flow::Verdict::Error as i32 => "ERROR".to_string(),
            _ => "UNKNOWN".to_string(),
        };

        let protocol = if let Some(_ip) = &flow.ip {
            if let Some(l4) = &flow.l4 {
                if let Some(proto) = &l4.protocol {
                    match proto {
                        crate::api::flow::layer4::Protocol::Tcp(_) => "TCP".to_string(),
                        crate::api::flow::layer4::Protocol::Udp(_) => "UDP".to_string(),
                        _ => "UNKNOWN".to_string(),
                    }
                } else {
                    "UNKNOWN".to_string()
                }
            } else {
                "UNKNOWN".to_string()
            }
        } else {
            "UNKNOWN".to_string()
        };

        Some(FlowData {
            id: 0,
            timestamp,
            source: SourceInfo {
                namespace: source_ns,
                pod: source_pod,
                ip: source_ip,
                port: source_port,
            },
            destination: DestInfo {
                namespace: dest_ns,
                pod: dest_pod,
                ip: dest_ip,
                port: dest_port,
            },
            protocol,
            verdict,
        })
    } else {
        None
    }
}

/// Start the gRPC client and stream flows
async fn start_grpc_stream(
    address: &str,
    port: u16,
    sender: broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let endpoint = format!("http://{}:{}", address, port);
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
    println!("Streaming flows to web UI and console...");
    println!("Press Ctrl+C to stop\n");

    let mut counter: u64 = 0;
    let formatter = FlowFormatter::new(false);

    while let Some(response) = stream.message().await? {
        if let Some(mut flow_data) = parse_flow_to_web_data(&response) {
            counter += 1;
            flow_data.id = counter;
            
            if let Ok(json) = serde_json::to_string(&flow_data) {
                let _ = sender.send(json);
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

/// WebSocket connection handler
pub struct WsConn {
    receiver: broadcast::Receiver<String>,
}

impl actix::Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("WebSocket client connected");

        let mut receiver = self.receiver.resubscribe();
        
        ctx.run_interval(Duration::from_millis(100), move |_act: &mut WsConn, ctx: &mut <WsConn as actix::Actor>::Context| {
            loop {
                match receiver.try_recv() {
                    Ok(msg) => {
                        ctx.text(msg);
                    }
                    Err(broadcast::error::TryRecvError::Empty) => {
                        break;
                    }
                    Err(broadcast::error::TryRecvError::Lagged(_)) => {
                        println!("WebSocket receiver lagged, continuing...");
                    }
                    Err(broadcast::error::TryRecvError::Closed) => {
                        println!("WebSocket broadcast channel closed");
                        break;
                    }
                }
            }
        });
    }
}

impl actix::StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(_)) => {}
            Ok(ws::Message::Binary(_)) => {}
            Ok(ws::Message::Close(_)) => {
                println!("WebSocket client disconnected");
                ctx.close(None);
            }
            _ => {}
        }
    }
}

/// WebSocket handler
async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let receiver = req
        .app_data::<web::Data<Arc<broadcast::Receiver<String>>>>()
        .expect("Receiver should be in app data")
        .get_ref()
        .resubscribe();

    ws::start(
        WsConn { receiver },
        &req,
        stream,
    )
}

/// Serve the main HTML page
async fn serve_index() -> impl Responder {
    actix_web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/index.html"))
}

/// Serve CSS file
async fn serve_css() -> impl Responder {
    use actix_web::HttpResponse;
    HttpResponse::Ok()
        .content_type("text/css; charset=utf-8")
        .body(include_str!("../static/styles.css"))
}

/// Serve JavaScript file
async fn serve_js() -> impl Responder {
    use actix_web::HttpResponse;
    HttpResponse::Ok()
        .content_type("text/javascript; charset=utf-8")
        .body(include_str!("../static/app.js"))
}

pub async fn run_server(
    host: &str,
    port: u16,
    grpc_address: String,
    grpc_port: u16,
    _world_only: bool,
    _filter_ip: &[String],
    _exclude: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, receiver) = broadcast::channel::<String>(100);

    let grpc_sender = sender.clone();
    tokio::spawn(async move {
        if let Err(e) = start_grpc_stream(&grpc_address, grpc_port, grpc_sender).await {
            eprintln!("gRPC stream error: {}", e);
        }
    });

    let receiver = Arc::new(receiver);

    let server = HttpServer::new({
        let receiver = receiver.clone();
        move || {
            App::new()
                .app_data(web::Data::new(receiver.clone()))
                .route("/", web::get().to(serve_index))
                .route("/ws", web::get().to(ws_handler))
                .route("/styles.css", web::get().to(serve_css))
                .route("/app.js", web::get().to(serve_js))
        }
    })
    .bind((host, port))?
    .run();

    println!("Web UI available at http://{}:{}", host, port);
    println!("WebSocket endpoint: ws://{}:{}/ws", host, port);

    server.await?;

    Ok(())
}
