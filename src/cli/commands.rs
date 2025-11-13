use serde_json::{Value, json};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use super::{Command, ConfigAction, DashboardsAction, LogsAction, MonitorsAction};
use crate::datadog::DatadogClient;
use crate::error::{DatadogError, Result};
use crate::handlers;

pub async fn execute(command: &Command, client: Arc<DatadogClient>) -> Result<Value> {
    match command {
        Command::Metrics {
            query,
            from,
            to,
            max_points,
        } => {
            let params = json!({
                "query": query,
                "from": from,
                "to": to,
                "max_points": max_points,
            });
            handlers::metrics::MetricsHandler::query(client, &params).await
        }

        Command::Logs { action } => match action {
            LogsAction::Search {
                query,
                from,
                to,
                limit,
                tag_filter,
            } => {
                let params = json!({
                    "query": query,
                    "from": from,
                    "to": to,
                    "limit": limit,
                    "tag_filter": tag_filter,
                });
                handlers::logs::LogsHandler::search(client, &params).await
            }
            LogsAction::Aggregate { query, from, to } => {
                let params = json!({
                    "query": query,
                    "from": from,
                    "to": to,
                });
                handlers::logs_aggregate::LogsAggregateHandler::aggregate(client, &params).await
            }
            LogsAction::Timeseries {
                query,
                from,
                to,
                interval,
                aggregation,
                metric,
            } => {
                let params = json!({
                    "query": query,
                    "from": from,
                    "to": to,
                    "interval": interval,
                    "aggregation": aggregation,
                    "metric": metric,
                });
                handlers::logs_timeseries::LogsTimeseriesHandler::timeseries(client, &params).await
            }
        },

        Command::Monitors { action } => match action {
            MonitorsAction::List { tags, monitor_tags } => {
                let params = json!({
                    "tags": tags,
                    "monitor_tags": monitor_tags,
                });
                handlers::monitors::MonitorsHandler::list(client, &params).await
            }
            MonitorsAction::Get { monitor_id } => {
                let params = json!({
                    "monitor_id": monitor_id,
                });
                handlers::monitors::MonitorsHandler::get(client, &params).await
            }
        },

        Command::Events {
            from,
            to,
            priority,
            sources,
            tags,
        } => {
            let params = json!({
                "from": from,
                "to": to,
                "priority": priority,
                "sources": sources,
                "tags": tags,
            });
            handlers::events::EventsHandler::query(client, &params).await
        }

        Command::Hosts {
            filter,
            from,
            sort_field,
            sort_dir,
            start,
            count,
            tag_filter,
        } => {
            let params = json!({
                "filter": filter,
                "from": from,
                "sort_field": sort_field,
                "sort_dir": sort_dir,
                "start": start,
                "count": count,
                "tag_filter": tag_filter,
            });
            handlers::hosts::HostsHandler::list(client, &params).await
        }

        Command::Dashboards { action } => match action {
            DashboardsAction::List => {
                handlers::dashboards::DashboardsHandler::list(client, &json!({})).await
            }
            DashboardsAction::Get { dashboard_id } => {
                let params = json!({
                    "dashboard_id": dashboard_id,
                });
                handlers::dashboards::DashboardsHandler::get(client, &params).await
            }
        },

        Command::Spans {
            query,
            from,
            to,
            limit,
            cursor,
            sort,
            tag_filter,
            full_stack_trace,
        } => {
            let params = json!({
                "query": query,
                "from": from,
                "to": to,
                "limit": limit,
                "cursor": cursor,
                "sort": sort,
                "tag_filter": tag_filter,
                "full_stack_trace": full_stack_trace,
            });
            handlers::spans::SpansHandler::list(client, &params).await
        }

        Command::Services { env } => {
            let params = json!({
                "env": env,
            });
            handlers::services::ServicesHandler::list(client, &params).await
        }

        Command::Rum {
            query,
            from,
            to,
            limit,
            cursor,
            sort,
            tag_filter,
            full_stack_trace,
        } => {
            let params = json!({
                "query": query,
                "from": from,
                "to": to,
                "limit": limit,
                "cursor": cursor,
                "sort": sort,
                "tag_filter": tag_filter,
                "full_stack_trace": full_stack_trace,
            });
            handlers::rum::RumHandler::search_events(client, &params).await
        }

        Command::Config { .. } => {
            unreachable!("Config command is handled separately")
        }
    }
}

fn get_global_config_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("datadog-cli")
        .join("config")
}

fn get_local_config_path() -> PathBuf {
    PathBuf::from(".env")
}

fn mask_secret(value: &str) -> String {
    if value.len() <= 8 {
        "*".repeat(value.len())
    } else {
        format!("{}...{}", &value[..4], &value[value.len() - 4..])
    }
}

pub fn handle_config(action: &ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Path { global } => {
            let path = if *global {
                get_global_config_path()
            } else {
                get_local_config_path()
            };
            println!("{}", path.display());
        }

        ConfigAction::Show => {
            println!("Current Configuration:");
            println!();

            if let Ok(api_key) = env::var("DD_API_KEY") {
                println!("  DD_API_KEY:  {} (from env)", mask_secret(&api_key));
            } else {
                println!("  DD_API_KEY:  not set");
            }

            if let Ok(app_key) = env::var("DD_APP_KEY") {
                println!("  DD_APP_KEY:  {} (from env)", mask_secret(&app_key));
            } else {
                println!("  DD_APP_KEY:  not set");
            }

            if let Ok(site) = env::var("DD_SITE") {
                println!("  DD_SITE:     {}", site);
            } else {
                println!("  DD_SITE:     datadoghq.com (default)");
            }

            if let Ok(log_level) = env::var("LOG_LEVEL") {
                println!("  LOG_LEVEL:   {}", log_level);
            } else {
                println!("  LOG_LEVEL:   warn (default)");
            }

            if let Ok(tag_filter) = env::var("DD_TAG_FILTER") {
                println!("  DD_TAG_FILTER: {}", tag_filter);
            }
        }

        ConfigAction::List => {
            println!("Configuration Sources (in priority order):");
            println!();

            println!("1. Environment Variables:");
            for (key, value) in env::vars() {
                if key.starts_with("DD_") || key == "LOG_LEVEL" {
                    let display_value = if key.contains("KEY") {
                        mask_secret(&value)
                    } else {
                        value
                    };
                    println!("   {}={}", key, display_value);
                }
            }
            println!();

            let local_path = get_local_config_path();
            println!("2. Local Config: {}", local_path.display());
            if local_path.exists() {
                println!("   Status: exists");
            } else {
                println!("   Status: not found");
            }
            println!();

            let global_path = get_global_config_path();
            println!("3. Global Config: {}", global_path.display());
            if global_path.exists() {
                println!("   Status: exists");
            } else {
                println!("   Status: not found");
            }
        }

        ConfigAction::Edit { global } => {
            let path = if *global {
                get_global_config_path()
            } else {
                get_local_config_path()
            };

            if !path.exists() {
                eprintln!("Config file not found: {}", path.display());
                eprintln!();
                eprintln!("Create it with:");
                if *global {
                    eprintln!("  mkdir -p ~/.config/datadog-cli");
                    eprintln!("  cat > {} << EOF", path.display());
                } else {
                    eprintln!("  cat > .env << EOF");
                }
                eprintln!("DD_API_KEY=your_api_key");
                eprintln!("DD_APP_KEY=your_app_key");
                eprintln!("DD_SITE=datadoghq.com");
                eprintln!("LOG_LEVEL=error");
                eprintln!("EOF");
                return Err(DatadogError::InvalidInput(
                    "Config file not found".to_string(),
                ));
            }

            let editor = env::var("EDITOR").unwrap_or_else(|_| {
                if cfg!(target_os = "macos") {
                    "open".to_string()
                } else if cfg!(target_os = "windows") {
                    "notepad".to_string()
                } else {
                    "vi".to_string()
                }
            });

            let status = std::process::Command::new(&editor)
                .arg(&path)
                .status()
                .map_err(|e| {
                    DatadogError::InvalidInput(format!("Failed to launch editor: {}", e))
                })?;

            if !status.success() {
                return Err(DatadogError::InvalidInput(
                    "Editor exited with error".to_string(),
                ));
            }
        }
    }

    Ok(())
}
