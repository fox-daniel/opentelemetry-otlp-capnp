use opentelemetry_otlp_capnp::SpanReceiver;

const TEST_ADDRESS: &str = "127.0.0.1:4317";

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _span_receiver = SpanReceiver::new(TEST_ADDRESS)
        .start()
        .map_err(|e| format!("Failed to start SpanReceiver: {e}"))?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}
