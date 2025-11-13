use clap::Parser;
use dotenvy::dotenv;
use std::env;
use std::process;

#[tokio::main]
async fn main() {
    dotenv().ok();

    env_logger::Builder::from_env(env_logger::Env::default().filter_or(
        "RUST_LOG",
        env::var("LOG_LEVEL").unwrap_or_else(|_| "warn".to_string()),
    ))
    .init();

    let cli = datadog_cli::cli::Cli::parse();

    if let Err(e) = datadog_cli::cli::run(cli).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
