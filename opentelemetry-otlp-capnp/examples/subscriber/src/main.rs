// use subscriber::CapnpSubscriber;
use tracing::instrument;
use tracing_subscriber::fmt::format::FmtSpan;

const TEST_ADDRESS: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_telemetry_receiver().await?;
    run_app().await?;
    Ok(())
}

async fn run_telemetry_receiver() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

async fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    // let subscriber = CapnpSubscriber;
    let subscriber = tracing_subscriber::fmt()
        .with_span_events(FmtSpan::FULL)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    let s = "17";
    say_hello(s);
    Ok(())
}

#[instrument]
fn say_hello(s: &str) {
    println!("hello {}", s);
}
