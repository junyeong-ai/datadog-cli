mod commands;
mod output;

use clap::{Parser, Subcommand};
use std::sync::Arc;

use crate::config::Config;
use crate::datadog::DatadogClient;
use crate::error::Result;

#[derive(Parser)]
#[command(name = "datadog")]
#[command(version)]
#[command(about = "Datadog CLI", long_about = None)]
pub struct Cli {
    #[arg(long, default_value = "json", help = "Output format: json, jsonl, table")]
    pub format: String,

    #[arg(short = 'q', long, help = "Quiet mode")]
    pub quiet: bool,

    #[arg(short = 'v', long, help = "Verbose mode")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Query time series metrics")]
    Metrics {
        #[arg(help = "Metrics query")]
        query: String,

        #[arg(long, default_value = "1 hour ago")]
        from: String,

        #[arg(long, default_value = "now")]
        to: String,

        #[arg(long)]
        max_points: Option<usize>,
    },

    #[command(about = "Log operations")]
    Logs {
        #[command(subcommand)]
        action: LogsAction,
    },

    #[command(about = "Monitor operations")]
    Monitors {
        #[command(subcommand)]
        action: MonitorsAction,
    },

    #[command(about = "Query events")]
    Events {
        #[arg(long, default_value = "1 hour ago")]
        from: String,

        #[arg(long, default_value = "now")]
        to: String,

        #[arg(long)]
        priority: Option<String>,

        #[arg(long)]
        sources: Option<String>,

        #[arg(long)]
        tags: Option<String>,
    },

    #[command(about = "List infrastructure hosts")]
    Hosts {
        #[arg(long)]
        filter: Option<String>,

        #[arg(long, default_value = "1 hour ago")]
        from: String,

        #[arg(long)]
        sort_field: Option<String>,

        #[arg(long)]
        sort_dir: Option<String>,

        #[arg(long, default_value = "0")]
        start: usize,

        #[arg(long, default_value = "100")]
        count: usize,

        #[arg(long)]
        tag_filter: Option<String>,
    },

    #[command(about = "Dashboard operations")]
    Dashboards {
        #[command(subcommand)]
        action: DashboardsAction,
    },

    #[command(about = "Search APM spans")]
    Spans {
        #[arg(default_value = "*")]
        query: String,

        #[arg(long)]
        from: String,

        #[arg(long)]
        to: String,

        #[arg(long, default_value = "10")]
        limit: usize,

        #[arg(long)]
        cursor: Option<String>,

        #[arg(long)]
        sort: Option<String>,

        #[arg(long)]
        tag_filter: Option<String>,

        #[arg(long)]
        full_stack_trace: bool,
    },

    #[command(about = "List services")]
    Services {
        #[arg(long)]
        env: Option<String>,
    },

    #[command(about = "Search RUM events")]
    Rum {
        #[arg(default_value = "*")]
        query: String,

        #[arg(long, default_value = "1 hour ago")]
        from: String,

        #[arg(long, default_value = "now")]
        to: String,

        #[arg(long, default_value = "10")]
        limit: usize,

        #[arg(long)]
        cursor: Option<String>,

        #[arg(long)]
        sort: Option<String>,

        #[arg(long)]
        tag_filter: Option<String>,

        #[arg(long)]
        full_stack_trace: bool,
    },

    #[command(about = "Config management")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum LogsAction {
    #[command(about = "Search logs")]
    Search {
        #[arg(help = "Log search query")]
        query: String,

        #[arg(long, default_value = "1 hour ago")]
        from: String,

        #[arg(long, default_value = "now")]
        to: String,

        #[arg(long, default_value = "10")]
        limit: usize,

        #[arg(long)]
        tag_filter: Option<String>,
    },

    #[command(about = "Aggregate logs")]
    Aggregate {
        #[arg(default_value = "*")]
        query: String,

        #[arg(long)]
        from: String,

        #[arg(long)]
        to: String,
    },

    #[command(about = "Generate log timeseries")]
    Timeseries {
        #[arg(default_value = "*")]
        query: String,

        #[arg(long)]
        from: String,

        #[arg(long)]
        to: String,

        #[arg(long, default_value = "1h")]
        interval: String,

        #[arg(long, default_value = "count")]
        aggregation: String,

        #[arg(long)]
        metric: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum MonitorsAction {
    #[command(about = "List monitors")]
    List {
        #[arg(long)]
        tags: Option<String>,

        #[arg(long)]
        monitor_tags: Option<String>,
    },

    #[command(about = "Get monitor details")]
    Get {
        #[arg(help = "Monitor ID")]
        monitor_id: i64,
    },
}

#[derive(Subcommand)]
pub enum DashboardsAction {
    #[command(about = "List dashboards")]
    List,

    #[command(about = "Get dashboard details")]
    Get {
        #[arg(help = "Dashboard ID")]
        dashboard_id: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "Initialize config file")]
    Init,

    #[command(about = "Show current config")]
    Show,

    #[command(about = "Show config file path")]
    Path,
}

pub async fn run(cli: Cli) -> Result<()> {
    if let Command::Config { ref action } = cli.command {
        return commands::handle_config(action);
    }

    let config = Config::load()?;
    let client = Arc::new(DatadogClient::new(
        config.api_key.clone(),
        config.app_key.clone(),
        Some(config.site.clone()),
    )?);

    let format = output::Format::from_str(&cli.format)
        .map_err(crate::error::DatadogError::InvalidInput)?;

    let result = commands::execute(&cli.command, client).await?;
    output::print(&result, &format)?;

    Ok(())
}
