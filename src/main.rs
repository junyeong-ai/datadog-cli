use clap::Parser;
use std::process;

#[tokio::main]
async fn main() {
    let cli = datadog_cli::cli::Cli::parse();

    init_logging(cli.verbose);

    if let Err(e) = datadog_cli::cli::run(cli).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn init_logging(verbose: bool) {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::from_default_env().add_directive("warn".parse().unwrap())
    };

    fmt().with_env_filter(filter).with_target(false).init();
}
