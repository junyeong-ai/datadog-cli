use chrono::{DateTime, Utc};
use serde_json::Value;

use super::{
    Command, ConfigAction, DashboardsAction, ErrorTrackingAction, IncidentsAction, LogsAction,
    MonitorsAction, SloAction,
};
use crate::config::Config;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers;

pub async fn execute(
    command: &Command,
    client: &DatadogClient,
    now: DateTime<Utc>,
) -> Result<Value> {
    match command {
        Command::Mcp { .. } => unreachable!("MCP commands use their explicit transport"),
        Command::Capabilities { .. } => {
            unreachable!("Capabilities are handled without credentials")
        }
        Command::Query(_) => {
            unreachable!("Reviewed queries are prepared before credential execution")
        }
        Command::Metrics(args) => handlers::metrics::MetricsHandler::query(client, args, now).await,

        Command::Timeseries(args) => {
            handlers::metrics::MetricsHandler::timeseries(client, args, now).await
        }

        Command::Scalar(args) => handlers::metrics::MetricsHandler::scalar(client, args, now).await,

        Command::Logs { action } => match action {
            LogsAction::Search(args) => {
                handlers::logs::LogsHandler::search(client, args, now).await
            }
            LogsAction::Aggregate(args) => {
                handlers::logs::LogsHandler::aggregate(client, args, now).await
            }
            LogsAction::Timeseries(args) => {
                handlers::logs::LogsHandler::timeseries(client, args, now).await
            }
        },

        Command::Monitors { action } => match action {
            MonitorsAction::List(args) => {
                handlers::monitors::MonitorsHandler::list(client, args).await
            }
            MonitorsAction::Get { monitor_id } => {
                handlers::monitors::MonitorsHandler::get(client, *monitor_id).await
            }
        },

        Command::Events(args) => handlers::events::EventsHandler::search(client, args, now).await,

        Command::Hosts(args) => handlers::hosts::HostsHandler::list(client, args, now).await,

        Command::Dashboards { action } => match action {
            DashboardsAction::List(args) => {
                handlers::dashboards::DashboardsHandler::list(client, args).await
            }
            DashboardsAction::Get { dashboard_id } => {
                handlers::dashboards::DashboardsHandler::get(client, dashboard_id).await
            }
        },

        Command::Spans(args) => handlers::spans::SpansHandler::list(client, args, now).await,

        Command::Services(args) => handlers::services::ServicesHandler::list(client, args).await,

        Command::Rum(args) => handlers::rum::RumHandler::search_events(client, args, now).await,

        Command::Slo { action } => match action {
            SloAction::List(args) => handlers::slo::SloHandler::list(client, args).await,
            SloAction::Get { slo_id } => handlers::slo::SloHandler::get(client, slo_id).await,
        },

        Command::Incidents { action } => match action {
            IncidentsAction::List(args) => {
                handlers::incidents::IncidentsHandler::list(client, args).await
            }
            IncidentsAction::Get(args) => {
                handlers::incidents::IncidentsHandler::get(client, args).await
            }
        },

        Command::ErrorTracking { action } => match action {
            ErrorTrackingAction::Search(args) => {
                handlers::error_tracking::ErrorTrackingHandler::search(client, args, now).await
            }
            ErrorTrackingAction::Get { issue_id } => {
                handlers::error_tracking::ErrorTrackingHandler::get(client, issue_id).await
            }
        },

        Command::Downtimes(args) => handlers::downtimes::DowntimesHandler::list(client, args).await,

        Command::Audit(args) => handlers::audit::AuditHandler::search(client, args, now).await,

        Command::Teams(args) => handlers::teams::TeamsHandler::list(client, args).await,

        Command::LlmObs(args) => handlers::llm_obs::LlmObsHandler::search(client, args, now).await,

        Command::Config { .. } => {
            unreachable!("Config command is handled separately")
        }
    }
}

pub fn handle_config(action: &ConfigAction, cli: &super::Cli) -> Result<()> {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    match action {
        ConfigAction::Init => {
            let path = Config::init()?;
            writeln!(stdout, "Created: {}", path.display())?;
        }
        ConfigAction::Show => {
            let output = Config::show_with_options(cli.config_options())?;
            writeln!(stdout, "{}", output)?;
        }
        ConfigAction::Path => {
            let path = Config::active_config_path()?;
            writeln!(stdout, "{}", path.display())?;
        }
        ConfigAction::Edit => {
            Config::edit()?;
        }
    }

    Ok(())
}
