// use subscriber::CapnpSubscriber;
use basic_subscriber::CapnpSubscriber;
use tracing::instrument;
use tracing_subscriber::fmt::format::FmtSpan;

const TELEMETRY_RECEIVER: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = TELEMETRY_RECEIVER
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();
    run_telemetry_receiver(&addr).await?;
    run_app(&addr).await?;
    Ok(())
}

async fn run_telemetry_receiver(addr: &SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

async fn run_app(addr: &SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = CapnpSubscriber::new();
    // let subscriber = tracing_subscriber::fmt()
    //     .with_span_events(FmtSpan::FULL)
    //     .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    let s = "17";
    say_hello(s);
    Ok(())
}

#[instrument]
fn say_hello(s: &str) {
    println!("hello {}", s);
}
