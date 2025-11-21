pub mod app;
pub mod receiver;

const ADDRESS: &str = "127.0.0.1:8080";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("run the command: `cargo run receiver` or `cargo run app`");
    }
    match &args[1][..] {
        "app" => return app::main(),
        "receiver" => return receiver::main(),
        _ => (),
    }
    Ok(())
}
