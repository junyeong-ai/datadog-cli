mod commands;
mod output;

use clap::{Parser, Subcommand};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;

#[derive(Parser)]
#[command(name = "datadog")]
#[command(version)]
#[command(about = "Datadog CLI - High-performance observability tool", long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
pub struct GlobalOpts {
    #[arg(long, env = "DD_API_KEY", help = "Datadog API key")]
    pub api_key: Option<String>,

    #[arg(long, env = "DD_APP_KEY", help = "Datadog application key")]
    pub app_key: Option<String>,

    #[arg(
        long,
        env = "DD_SITE",
        default_value = "datadoghq.com",
        help = "Datadog site"
    )]
    pub site: String,

    #[arg(
        long,
        default_value = "json",
        help = "Output format: json, jsonl, table"
    )]
    pub format: String,

    #[arg(short = 'q', long, help = "Quiet mode (errors only)")]
    pub quiet: bool,

    #[arg(short = 'v', long, help = "Verbose mode")]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Query time series metrics")]
    Metrics {
        #[arg(help = "Metrics query (e.g., 'avg:system.cpu.user{*}')")]
        query: String,

        #[arg(long, default_value = "1 hour ago", help = "Start time")]
        from: String,

        #[arg(long, default_value = "now", help = "End time")]
        to: String,

        #[arg(long, help = "Maximum number of data points")]
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
        #[arg(long, default_value = "1 hour ago", help = "Start time")]
        from: String,

        #[arg(long, default_value = "now", help = "End time")]
        to: String,

        #[arg(long, help = "Priority filter (normal, low)")]
        priority: Option<String>,

        #[arg(long, help = "Sources filter")]
        sources: Option<String>,

        #[arg(long, help = "Tags filter")]
        tags: Option<String>,
    },

    #[command(about = "List infrastructure hosts")]
    Hosts {
        #[arg(long, help = "Host filter query")]
        filter: Option<String>,

        #[arg(long, default_value = "1 hour ago", help = "From time")]
        from: String,

        #[arg(long, help = "Sort field")]
        sort_field: Option<String>,

        #[arg(long, help = "Sort direction (asc, desc)")]
        sort_dir: Option<String>,

        #[arg(long, default_value = "0", help = "Starting index")]
        start: usize,

        #[arg(long, default_value = "100", help = "Number of hosts (max 1000)")]
        count: usize,

        #[arg(long, help = "Tag filter")]
        tag_filter: Option<String>,
    },

    #[command(about = "Dashboard operations")]
    Dashboards {
        #[command(subcommand)]
        action: DashboardsAction,
    },

    #[command(about = "Search APM spans")]
    Spans {
        #[arg(default_value = "*", help = "Spans search query")]
        query: String,

        #[arg(long, help = "Start time")]
        from: String,

        #[arg(long, help = "End time")]
        to: String,

        #[arg(long, default_value = "10", help = "Maximum number of spans")]
        limit: usize,

        #[arg(long, help = "Pagination cursor")]
        cursor: Option<String>,

        #[arg(long, help = "Sort order (e.g., 'timestamp')")]
        sort: Option<String>,

        #[arg(long, help = "Tag filter")]
        tag_filter: Option<String>,

        #[arg(long, help = "Include full stack traces")]
        full_stack_trace: bool,
    },

    #[command(about = "List services from service catalog")]
    Services {
        #[arg(long, help = "Filter by environment")]
        env: Option<String>,
    },

    #[command(about = "Search RUM events")]
    Rum {
        #[arg(default_value = "*", help = "RUM search query")]
        query: String,

        #[arg(long, default_value = "1 hour ago", help = "Start time")]
        from: String,

        #[arg(long, default_value = "now", help = "End time")]
        to: String,

        #[arg(long, default_value = "10", help = "Maximum number of events")]
        limit: usize,

        #[arg(long, help = "Pagination cursor")]
        cursor: Option<String>,

        #[arg(long, help = "Sort order")]
        sort: Option<String>,

        #[arg(long, help = "Tag filter")]
        tag_filter: Option<String>,

        #[arg(long, help = "Include full stack traces")]
        full_stack_trace: bool,
    },

    #[command(about = "Configuration management")]
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

        #[arg(long, default_value = "1 hour ago", help = "Start time")]
        from: String,

        #[arg(long, default_value = "now", help = "End time")]
        to: String,

        #[arg(long, default_value = "10", help = "Maximum number of logs")]
        limit: usize,

        #[arg(long, help = "Tag filter")]
        tag_filter: Option<String>,
    },

    #[command(about = "Aggregate logs")]
    Aggregate {
        #[arg(default_value = "*", help = "Log search query")]
        query: String,

        #[arg(long, help = "Start time")]
        from: String,

        #[arg(long, help = "End time")]
        to: String,
    },

    #[command(about = "Generate log timeseries")]
    Timeseries {
        #[arg(default_value = "*", help = "Log search query")]
        query: String,

        #[arg(long, help = "Start time")]
        from: String,

        #[arg(long, help = "End time")]
        to: String,

        #[arg(
            long,
            default_value = "1h",
            help = "Time interval (e.g., '1m', '5m', '1h')"
        )]
        interval: String,

        #[arg(long, default_value = "count", help = "Aggregation type")]
        aggregation: String,

        #[arg(long, help = "Field to aggregate on")]
        metric: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum MonitorsAction {
    #[command(about = "List monitors")]
    List {
        #[arg(long, help = "Filter by tags")]
        tags: Option<String>,

        #[arg(long, help = "Filter by monitor tags")]
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
    #[command(about = "Show configuration file path")]
    Path {
        #[arg(long, help = "Show global config path")]
        global: bool,
    },

    #[command(about = "Show current configuration (with masked secrets)")]
    Show,

    #[command(about = "List all configuration sources")]
    List,

    #[command(about = "Edit configuration file")]
    Edit {
        #[arg(long, help = "Edit global config")]
        global: bool,
    },
}

pub async fn run(cli: Cli) -> Result<()> {
    if let Command::Config { ref action } = cli.command {
        return commands::handle_config(action);
    }

    let api_key = cli.global.api_key.ok_or_else(|| {
        crate::error::DatadogError::InvalidInput(
            "DD_API_KEY is required (set via --api-key or DD_API_KEY env var)".to_string(),
        )
    })?;

    let app_key = cli.global.app_key.ok_or_else(|| {
        crate::error::DatadogError::InvalidInput(
            "DD_APP_KEY is required (set via --app-key or DD_APP_KEY env var)".to_string(),
        )
    })?;

    let client = Arc::new(DatadogClient::new(api_key, app_key, Some(cli.global.site))?);

    let format = output::Format::from_str(&cli.global.format)
        .map_err(crate::error::DatadogError::InvalidInput)?;

    let result = commands::execute(&cli.command, client).await?;

    output::print(&result, &format)?;

    Ok(())
}
