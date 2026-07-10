use clap::Parser;
use datadog_cli::error::DatadogError;
use std::io::ErrorKind;
use std::process;

#[tokio::main]
async fn main() {
    let cli = datadog_cli::cli::Cli::parse();

    init_logging(cli.verbose);

    if let Err(e) = datadog_cli::cli::run(cli).await {
        // A closed stdout (e.g. piping into `head`) is normal pipeline
        // behavior, not an error.
        if let DatadogError::IoError(ref io) = e
            && io.kind() == ErrorKind::BrokenPipe
        {
            process::exit(0);
        }

        eprintln!("Error: {}", e);
        process::exit(e.exit_code());
    }
}

fn init_logging(verbose: bool) {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::from_default_env().add_directive("warn".parse().unwrap())
    };

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();
}
