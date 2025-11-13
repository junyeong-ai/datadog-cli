use clap::Parser;
use std::process;

#[tokio::main]
async fn main() {
    env_logger::init();

    let cli = datadog_cli::cli::Cli::parse();

    if let Err(e) = datadog_cli::cli::run(cli).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
