use serde_json::Value;

use super::{
    Command, ConfigAction, DashboardsAction, ErrorTrackingAction, IncidentsAction, LogsAction,
    MonitorsAction, SloAction,
};
use crate::config::Config;
use crate::datadog::DatadogClient;
use crate::error::{DatadogError, Result};
use crate::handlers;

pub async fn execute(command: &Command, client: &DatadogClient) -> Result<Value> {
    match command {
        Command::Metrics(args) => handlers::metrics::MetricsHandler::query(client, args).await,

        Command::Timeseries(args) => {
            handlers::metrics::MetricsHandler::timeseries(client, args).await
        }

        Command::Scalar(args) => handlers::metrics::MetricsHandler::scalar(client, args).await,

        Command::Logs { action } => match action {
            LogsAction::Search(args) => handlers::logs::LogsHandler::search(client, args).await,
            LogsAction::Aggregate(args) => {
                handlers::logs::LogsHandler::aggregate(client, args).await
            }
            LogsAction::Timeseries(args) => {
                handlers::logs::LogsHandler::timeseries(client, args).await
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

        Command::Events(args) => handlers::events::EventsHandler::search(client, args).await,

        Command::Hosts(args) => handlers::hosts::HostsHandler::list(client, args).await,

        Command::Dashboards { action } => match action {
            DashboardsAction::List(args) => {
                handlers::dashboards::DashboardsHandler::list(client, args).await
            }
            DashboardsAction::Get { dashboard_id } => {
                handlers::dashboards::DashboardsHandler::get(client, dashboard_id).await
            }
        },

        Command::Spans(args) => handlers::spans::SpansHandler::list(client, args).await,

        Command::Services(args) => handlers::services::ServicesHandler::list(client, args).await,

        Command::Rum(args) => handlers::rum::RumHandler::search_events(client, args).await,

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
                handlers::error_tracking::ErrorTrackingHandler::search(client, args).await
            }
            ErrorTrackingAction::Get { issue_id } => {
                handlers::error_tracking::ErrorTrackingHandler::get(client, issue_id).await
            }
        },

        Command::Downtimes(args) => handlers::downtimes::DowntimesHandler::list(client, args).await,

        Command::Audit(args) => handlers::audit::AuditHandler::search(client, args).await,

        Command::Teams(args) => handlers::teams::TeamsHandler::list(client, args).await,

        Command::LlmObs(args) => handlers::llm_obs::LlmObsHandler::search(client, args).await,

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
