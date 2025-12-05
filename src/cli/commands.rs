use serde_json::{Value, json};
use std::sync::Arc;

use super::{Command, ConfigAction, DashboardsAction, LogsAction, MonitorsAction};
use crate::config::Config;
use crate::datadog::DatadogClient;
use crate::error::{DatadogError, Result};
use crate::handlers;

pub async fn execute(
    command: &Command,
    client: Arc<DatadogClient>,
    config: &Config,
) -> Result<Value> {
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
                cursor,
                sort,
                tag_filter,
            } => {
                let params = json!({
                    "query": query,
                    "from": from,
                    "to": to,
                    "limit": limit,
                    "cursor": cursor,
                    "sort": sort,
                    "tag_filter": tag_filter.as_ref().or(config.defaults.tag_filter.as_ref()),
                });
                handlers::logs::LogsHandler::search(client, &params).await
            }
            LogsAction::Aggregate { query, from, to } => {
                let params = json!({
                    "query": query,
                    "from": from,
                    "to": to,
                });
                handlers::logs::LogsHandler::aggregate(client, &params).await
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
                handlers::logs::LogsHandler::timeseries(client, &params).await
            }
        },

        Command::Monitors { action } => match action {
            MonitorsAction::List {
                tags,
                monitor_tags,
                page,
                page_size,
            } => {
                let params = json!({
                    "tags": tags,
                    "monitor_tags": monitor_tags,
                    "page": page,
                    "page_size": page_size,
                });
                handlers::monitors::MonitorsHandler::list(client, &params).await
            }
            MonitorsAction::Get { monitor_id } => {
                let params = json!({ "monitor_id": monitor_id });
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
                "tag_filter": tag_filter.as_ref().or(config.defaults.tag_filter.as_ref()),
            });
            handlers::hosts::HostsHandler::list(client, &params).await
        }

        Command::Dashboards { action } => match action {
            DashboardsAction::List {
                count,
                start,
                filter_shared,
                filter_deleted,
            } => {
                let params = json!({
                    "count": count,
                    "start": start,
                    "filter_shared": filter_shared,
                    "filter_deleted": filter_deleted,
                });
                handlers::dashboards::DashboardsHandler::list(client, &params).await
            }
            DashboardsAction::Get { dashboard_id } => {
                let params = json!({ "dashboard_id": dashboard_id });
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
                "tag_filter": tag_filter.as_ref().or(config.defaults.tag_filter.as_ref()),
                "full_stack_trace": full_stack_trace,
            });
            handlers::spans::SpansHandler::list(client, &params).await
        }

        Command::Services {
            env,
            page_size,
            page,
        } => {
            let params = json!({
                "env": env,
                "page_size": page_size,
                "page": page,
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
                "tag_filter": tag_filter.as_ref().or(config.defaults.tag_filter.as_ref()),
                "full_stack_trace": full_stack_trace,
            });
            handlers::rum::RumHandler::search_events(client, &params).await
        }

        Command::Config { .. } => {
            unreachable!("Config command is handled separately")
        }
    }
}

pub fn handle_config(action: &ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Init => {
            let path = Config::init()?;
            println!("Created: {}", path.display());
        }
        ConfigAction::Show => {
            let output = Config::show()?;
            println!("{}", output);
        }
        ConfigAction::Path => {
            let path = Config::global_config_path()
                .ok_or_else(|| DatadogError::InvalidInput("Cannot determine config path".into()))?;
            println!("{}", path.display());
        }
        ConfigAction::Edit => {
            Config::edit()?;
        }
    }

    Ok(())
}
