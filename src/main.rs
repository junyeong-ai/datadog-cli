use clap::Parser;
use datadog_cli::error::{DatadogError, ErrorFormat};
use std::io::ErrorKind;
use std::process;

mod diagnostics;

#[tokio::main]
async fn main() {
    let cli = datadog_cli::cli::Cli::parse();
    let error_format = cli.error_format;

    let result = async {
        init_logging(cli.verbose)?;
        if matches!(
            &cli.command,
            datadog_cli::cli::Command::Config {
                action: datadog_cli::cli::ConfigAction::Edit
            }
        ) {
            #[cfg(unix)]
            let _interrupts =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
            #[cfg(windows)]
            let _interrupts = tokio::signal::windows::ctrl_c()?;
            datadog_cli::cli::run(cli).await
        } else {
            tokio::select! {
                result = datadog_cli::cli::run(cli) => result,
                signal = tokio::signal::ctrl_c() => {
                    signal.map_err(DatadogError::IoError)?;
                    Err(DatadogError::Interrupted)
                }
            }
        }
    }
    .await;
    if let Err(e) = result {
        // A closed stdout (e.g. piping into `head`) is normal pipeline
        // behavior, not an error.
        if let DatadogError::IoError(ref io) = e
            && io.kind() == ErrorKind::BrokenPipe
        {
            process::exit(0);
        }

        if !matches!(e, DatadogError::Interrupted) {
            let message = match error_format {
                ErrorFormat::Text => format!("Error: {e}"),
                ErrorFormat::Json => e.diagnostic().to_string(),
            };
            let _ = diagnostics::write(&mut std::io::stderr().lock(), format_args!("{message}"));
        }
        process::exit(e.exit_code());
    }
}

fn logging_filter(
    verbose: bool,
    directives: &str,
) -> Result<tracing_subscriber::EnvFilter, DatadogError> {
    use tracing_subscriber::{EnvFilter, filter::LevelFilter};
    EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into())
        .parse(if verbose { "debug" } else { directives })
        .map_err(|error| DatadogError::InvalidInput(format!("Invalid RUST_LOG filter: {error}")))
}

fn init_logging(verbose: bool) -> Result<(), DatadogError> {
    let directives = match std::env::var("RUST_LOG") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => String::new(),
        Err(error) => {
            return Err(DatadogError::InvalidInput(format!(
                "Invalid RUST_LOG: {error}"
            )));
        }
    };
    tracing_subscriber::fmt()
        .with_env_filter(logging_filter(verbose, &directives)?)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|error| DatadogError::InvalidInput(format!("Cannot initialize logging: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logging_respects_environment_and_explicit_verbosity() {
        assert_eq!(logging_filter(false, "").unwrap().to_string(), "warn");
        assert_eq!(logging_filter(false, "debug").unwrap().to_string(), "debug");
        assert_eq!(logging_filter(true, "error").unwrap().to_string(), "debug");
        assert!(logging_filter(false, "datadog_cli=invalid").is_err());
    }
}
