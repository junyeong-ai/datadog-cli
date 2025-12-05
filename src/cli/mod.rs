mod commands;
mod output;

use clap::{Parser, Subcommand};
use std::sync::Arc;

use crate::config::Config;
use crate::datadog::DatadogClient;
use crate::error::Result;

const TIME_HELP: &str = "Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp";
const SORT_HELP: &str = "Sort order (use --sort=\"-timestamp\" for descending)";

#[derive(Parser)]
#[command(name = "datadog-cli")]
#[command(version)]
#[command(about = "High-performance Datadog CLI")]
pub struct Cli {
    #[arg(long, value_parser = ["json", "jsonl", "table"], help = "Output format (default from config)")]
    pub format: Option<String>,

    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    #[arg(long, env = "DD_API_KEY", global = true, hide_env_values = true)]
    pub api_key: Option<String>,

    #[arg(long, env = "DD_APP_KEY", global = true, hide_env_values = true)]
    pub app_key: Option<String>,

    #[arg(long, env = "DD_SITE", global = true)]
    pub site: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Query time series metrics")]
    Metrics {
        query: String,

        #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
        from: String,

        #[arg(long, default_value = "now", help = TIME_HELP)]
        to: String,

        #[arg(long, help = "Limit data points by auto-rollup")]
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
        #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
        from: String,

        #[arg(long, default_value = "now", help = TIME_HELP)]
        to: String,

        #[arg(long, help = "Filter by priority (low, normal)")]
        priority: Option<String>,

        #[arg(long, help = "Comma-separated source names")]
        sources: Option<String>,

        #[arg(long, help = "Comma-separated tags")]
        tags: Option<String>,
    },

    #[command(about = "List infrastructure hosts")]
    Hosts {
        #[arg(long, help = "Filter hosts by name, alias, or tag")]
        filter: Option<String>,

        #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
        from: String,

        #[arg(long, help = "Sort field (e.g., cpu, iowait, load)")]
        sort_field: Option<String>,

        #[arg(long, help = "Sort direction (asc, desc)")]
        sort_dir: Option<String>,

        #[arg(long, default_value = "0", help = "Pagination offset")]
        start: i32,

        #[arg(long, default_value = "100", help = "Results per page (max 1000)")]
        count: i32,

        #[arg(long, help = "Tag prefixes to include (default from config)")]
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

        #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
        from: String,

        #[arg(long, default_value = "now", help = TIME_HELP)]
        to: String,

        #[arg(long, default_value = "10")]
        limit: i32,

        #[arg(long, help = "Pagination cursor from previous response")]
        cursor: Option<String>,

        #[arg(long, help = SORT_HELP)]
        sort: Option<String>,

        #[arg(long, help = "Tag prefixes to include (default from config)")]
        tag_filter: Option<String>,

        #[arg(long, help = "Show full stack traces")]
        full_stack_trace: bool,
    },

    #[command(about = "List services from catalog")]
    Services {
        #[arg(long, help = "Filter by environment")]
        env: Option<String>,

        #[arg(long, default_value = "100")]
        page_size: i32,

        #[arg(long, default_value = "0")]
        page: i32,
    },

    #[command(about = "Search RUM events")]
    Rum {
        #[arg(default_value = "*")]
        query: String,

        #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
        from: String,

        #[arg(long, default_value = "now", help = TIME_HELP)]
        to: String,

        #[arg(long, default_value = "10")]
        limit: i32,

        #[arg(long, help = "Pagination cursor from previous response")]
        cursor: Option<String>,

        #[arg(long, help = SORT_HELP)]
        sort: Option<String>,

        #[arg(long, help = "Tag prefixes to include (default from config)")]
        tag_filter: Option<String>,

        #[arg(long, help = "Show full stack traces")]
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
        #[arg(default_value = "*")]
        query: String,

        #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
        from: String,

        #[arg(long, default_value = "now", help = TIME_HELP)]
        to: String,

        #[arg(long, default_value = "10")]
        limit: i32,

        #[arg(long, help = "Pagination cursor from previous response")]
        cursor: Option<String>,

        #[arg(long, help = SORT_HELP)]
        sort: Option<String>,

        #[arg(long, help = "Tag prefixes to include (default from config)")]
        tag_filter: Option<String>,
    },

    #[command(about = "Aggregate logs into buckets")]
    Aggregate {
        #[arg(default_value = "*")]
        query: String,

        #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
        from: String,

        #[arg(long, default_value = "now", help = TIME_HELP)]
        to: String,
    },

    #[command(about = "Generate log timeseries")]
    Timeseries {
        #[arg(default_value = "*")]
        query: String,

        #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
        from: String,

        #[arg(long, default_value = "now", help = TIME_HELP)]
        to: String,

        #[arg(long, default_value = "1h", help = "Rollup interval (e.g., 5m, 1h, 1d)")]
        interval: String,

        #[arg(long, default_value = "count", help = "Aggregation type (count, avg, sum, min, max)")]
        aggregation: String,

        #[arg(long, help = "Metric field for aggregation")]
        metric: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum MonitorsAction {
    #[command(about = "List monitors")]
    List {
        #[arg(long, help = "Filter by resource tags")]
        tags: Option<String>,

        #[arg(long, help = "Filter by monitor tags")]
        monitor_tags: Option<String>,

        #[arg(long, default_value = "0", help = "Page number")]
        page: i32,

        #[arg(long, default_value = "100", help = "Results per page")]
        page_size: i32,
    },

    #[command(about = "Get monitor details")]
    Get { monitor_id: i64 },
}

#[derive(Subcommand)]
pub enum DashboardsAction {
    #[command(about = "List dashboards")]
    List {
        #[arg(long, default_value = "100", help = "Results per page")]
        count: i32,

        #[arg(long, default_value = "0", help = "Pagination offset")]
        start: i32,

        #[arg(long, help = "Include shared dashboards only")]
        filter_shared: bool,

        #[arg(long, help = "Include deleted dashboards only")]
        filter_deleted: bool,
    },

    #[command(about = "Get dashboard details")]
    Get { dashboard_id: String },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "Initialize config file")]
    Init,

    #[command(about = "Show current config")]
    Show,

    #[command(about = "Show config file path")]
    Path,

    #[command(about = "Edit config file")]
    Edit,
}

pub async fn run(cli: Cli) -> Result<()> {
    if let Command::Config { ref action } = cli.command {
        return commands::handle_config(action);
    }

    let config = Config::load(cli.api_key, cli.app_key, cli.site)?;

    let client = Arc::new(DatadogClient::new(
        config.api_key().to_string(),
        config.app_key().to_string(),
        Some(config.site.clone()),
        config.network.timeout_secs,
        config.network.max_retries,
        config.defaults.tag_filter.clone(),
    )?);

    let format_str = cli.format.as_deref().unwrap_or(&config.defaults.format);
    let format =
        output::Format::from_str(format_str).map_err(crate::error::DatadogError::InvalidInput)?;

    let result = commands::execute(&cli.command, client, &config).await?;
    output::print(&result, &format)?;

    Ok(())
}
