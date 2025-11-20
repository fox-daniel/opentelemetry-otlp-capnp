use std::io::stdout;
// use subscriber::CapnpSubscriber;
use tracing::{event, instrument, Level};

const TEST_ADDRESS: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // let subscriber = CapnpSubscriber;
    // let (non_blocking, _guard) = tracing_appender::non_blocking(stdout());
    let subscriber = tracing_subscriber::fmt().finish();
    tracing::subscriber::set_global_default(subscriber)?;
    let s = "17";
    say_hello(s);
    Ok(())
}

#[instrument]
fn say_hello(s: &str) {
    event!(target: "say_hello", Level::INFO, "something");
    println!("hello {}", s);
}
