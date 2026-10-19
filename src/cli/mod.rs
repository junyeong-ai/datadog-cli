mod commands;
mod discovery;
mod query;
use crate::output;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::config::{AuthMethod, Config, ConfigOptions};
use crate::datadog::{DatadogClient, MAX_METRIC_QUERIES};
use crate::error::Result;

const TIME_HELP: &str =
    "Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp";
const SORT_HELP: &str = "Sort order (use --sort=\"-timestamp\" for descending)";

#[derive(Clone, Copy, ValueEnum)]
pub enum AuthProvider {
    Pup,
}

#[derive(Parser)]
#[command(name = "datadog-cli")]
#[command(version)]
#[command(about = "High-performance Datadog CLI")]
pub struct Cli {
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "text",
        help = "Final runtime error format on stderr; CLI usage errors remain text"
    )]
    pub error_format: crate::error::ErrorFormat,

    #[arg(long, global = true, value_parser = ["json", "jsonl", "table"], help = "Output format (default from config)")]
    pub format: Option<String>,

    #[arg(short = 'v', long, global = true, help = "Enable debug logging")]
    pub verbose: bool,

    #[arg(long, global = true, help = "API key (otherwise DD_API_KEY or config)")]
    pub api_key: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Application key (otherwise DD_APP_KEY or config)"
    )]
    pub app_key: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Bearer token: PAT, SAT, or OAuth access token"
    )]
    pub token: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Datadog site (otherwise DD_SITE or config)"
    )]
    pub site: Option<String>,

    #[arg(long, global = true, value_enum, conflicts_with = "auth_method", conflicts_with_all = ["api_key", "app_key", "token"], help = "Use Pup OAuth (compatibility option for --auth-method oauth)")]
    pub auth_provider: Option<AuthProvider>,

    #[arg(
        long,
        global = true,
        value_enum,
        help = "Authentication method (otherwise global config [auth].method)"
    )]
    pub auth_method: Option<AuthMethod>,

    #[arg(
        long,
        global = true,
        help = "Named Pup OAuth session (otherwise global config [auth].pup_org)"
    )]
    pub pup_org: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Execute a reviewed API query with offline schema validation")]
    Query(query::QueryArgs),
    #[command(
        about = "Discover reviewed API queries and their input schemas without authentication"
    )]
    Capabilities {
        #[command(subcommand)]
        action: discovery::CapabilitiesAction,
    },
    #[command(about = "Query time series metrics")]
    Metrics(MetricsArgs),

    #[command(about = "Query timeseries with formulas across multiple metric queries")]
    Timeseries(TimeseriesArgs),

    #[command(about = "Query scalar values (single aggregate per query) with formulas")]
    Scalar(ScalarArgs),

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
    Events(EventsArgs),

    #[command(about = "List infrastructure hosts")]
    Hosts(HostsArgs),

    #[command(about = "Dashboard operations")]
    Dashboards {
        #[command(subcommand)]
        action: DashboardsAction,
    },

    #[command(about = "Search APM spans")]
    Spans(SpansArgs),

    #[command(about = "List services from catalog")]
    Services(ServicesArgs),

    #[command(about = "Search RUM events")]
    Rum(RumArgs),

    #[command(about = "Service Level Objective operations")]
    Slo {
        #[command(subcommand)]
        action: SloAction,
    },

    #[command(about = "Incident operations (requires Incident Management)")]
    Incidents {
        #[command(subcommand)]
        action: IncidentsAction,
    },

    #[command(about = "Error Tracking operations")]
    ErrorTracking {
        #[command(subcommand)]
        action: ErrorTrackingAction,
    },

    #[command(about = "List downtimes")]
    Downtimes(DowntimesArgs),

    #[command(about = "Search audit trail events")]
    Audit(AuditArgs),

    #[command(about = "List teams")]
    Teams(TeamsArgs),

    #[command(about = "Search LLM Observability spans (preview API, subject to change)")]
    LlmObs(LlmObsArgs),

    #[command(about = "Config management")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum SloAction {
    #[command(about = "List SLOs")]
    List(SloListArgs),

    #[command(about = "Get SLO details")]
    Get { slo_id: String },
}

#[derive(Args)]
pub struct SloListArgs {
    #[arg(long, help = "Filter by SLO name or description text")]
    pub query: Option<String>,

    #[arg(long, help = "Filter by tags (e.g. \"team:web\")")]
    pub tags_query: Option<String>,

    #[arg(long, default_value = "100", value_parser = clap::value_parser!(i32).range(1..), help = "Results per page")]
    pub limit: i32,

    #[arg(long, default_value = "0", value_parser = clap::value_parser!(i32).range(0..), help = "Pagination offset")]
    pub offset: i32,
}

#[derive(Subcommand)]
pub enum IncidentsAction {
    #[command(about = "List incidents")]
    List(IncidentsListArgs),

    #[command(about = "Get incident details")]
    Get(IncidentsGetArgs),
}

#[derive(Args)]
pub struct IncidentsListArgs {
    #[arg(long, default_value = "25", value_parser = clap::value_parser!(i32).range(1..=100), help = "Results per page (max 100)")]
    pub count: i32,

    #[arg(long, default_value = "0", value_parser = clap::value_parser!(i32).range(0..), help = "Pagination offset")]
    pub start: i32,

    #[arg(long, value_parser = ["users", "attachments"], help = "Include related data")]
    pub include: Option<String>,
}

#[derive(Args)]
pub struct IncidentsGetArgs {
    pub incident_id: String,

    #[arg(long, value_parser = ["users", "attachments"], help = "Include related data")]
    pub include: Option<String>,
}

#[derive(Subcommand)]
pub enum ErrorTrackingAction {
    #[command(about = "Search error issues")]
    Search(ErrorTrackingSearchArgs),

    #[command(about = "Get error issue details")]
    Get { issue_id: String },
}

#[derive(Args)]
pub struct ErrorTrackingSearchArgs {
    #[arg(default_value = "*", help = "Issue search query (event search syntax)")]
    pub query: String,

    #[arg(long, default_value = "trace", value_parser = ["trace", "logs", "rum"], help = "Error track to search")]
    pub track: String,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,

    #[arg(long, value_parser = ["issue", "issue.assignee", "issue.case", "issue.team_owners"], help = "Include related data")]
    pub include: Option<String>,
}

#[derive(Args)]
pub struct DowntimesArgs {
    #[arg(long, help = "Only downtimes active right now")]
    pub current_only: bool,

    #[arg(long, default_value = "0", value_parser = clap::value_parser!(i32).range(0..), help = "Pagination offset")]
    pub start: i32,

    #[arg(long, default_value = "100", value_parser = clap::value_parser!(i32).range(1..), help = "Results per page")]
    pub count: i32,
}

#[derive(Args)]
pub struct AuditArgs {
    #[arg(default_value = "*", help = "Audit event search query")]
    pub query: String,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,

    #[arg(long, default_value = "10", value_parser = clap::value_parser!(i32).range(1..=1000))]
    pub limit: i32,

    #[arg(long, help = "Pagination cursor from previous response")]
    pub cursor: Option<String>,

    #[arg(long, value_parser = ["timestamp", "-timestamp"], help = SORT_HELP)]
    pub sort: Option<String>,
}

#[derive(Args)]
pub struct TeamsArgs {
    #[arg(long, help = "Search teams by keyword")]
    pub keyword: Option<String>,

    #[arg(long, help = "Only teams the caller belongs to")]
    pub me: bool,

    #[arg(long, default_value = "0", value_parser = clap::value_parser!(i32).range(0..), help = "Page number")]
    pub page: i32,

    #[arg(long, default_value = "100", value_parser = clap::value_parser!(i32).range(1..=100), help = "Results per page (max 100)")]
    pub page_size: i32,
}

#[derive(Args)]
pub struct LlmObsArgs {
    #[arg(conflicts_with_all = ["ml_app", "span_kind"], help = "Agent Observability query; conflicts with structured filters")]
    pub query: Option<String>,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,

    #[arg(long, default_value = "10", value_parser = clap::value_parser!(i32).range(1..=5000), help = "Results per page (CLI max 5000)")]
    pub limit: i32,

    #[arg(long, help = "Pagination cursor from previous response")]
    pub cursor: Option<String>,

    #[arg(long, help = SORT_HELP)]
    pub sort: Option<String>,

    #[arg(long, help = "Filter by ML application name")]
    pub ml_app: Option<String>,

    #[arg(long, help = "Filter by span kind (e.g. agent, workflow, llm, tool)")]
    pub span_kind: Option<String>,
}

#[derive(Args)]
pub struct MetricsArgs {
    pub query: String,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,
}

#[derive(Args)]
pub struct TimeseriesArgs {
    #[arg(required = true, num_args = 1..=MAX_METRIC_QUERIES, help = "Metric queries, auto-named a, b, c… for formulas")]
    pub queries: Vec<String>,

    #[arg(
        long,
        help = "Formula over query names (e.g. \"a / b * 100\"); repeatable"
    )]
    pub formula: Vec<String>,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,

    #[arg(long, value_parser = clap::value_parser!(i64).range(1..=i64::MAX / 1000), help = "Rollup interval in seconds")]
    pub interval: Option<i64>,
}

#[derive(Args)]
pub struct ScalarArgs {
    #[arg(required = true, num_args = 1..=MAX_METRIC_QUERIES, help = "Metric queries, auto-named a, b, c… for formulas")]
    pub queries: Vec<String>,

    #[arg(
        long,
        help = "Formula over query names (e.g. \"a / b * 100\"); repeatable"
    )]
    pub formula: Vec<String>,

    #[arg(long, default_value = "avg", value_parser = ["avg", "min", "max", "sum", "last", "percentile", "mean", "l2norm", "area"], help = "Aggregator applied over the time window")]
    pub aggregator: String,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,
}

#[derive(Args)]
pub struct EventsArgs {
    #[arg(
        default_value = "*",
        help = "Event search query (e.g. \"source:alert status:error\")"
    )]
    pub query: String,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,

    #[arg(long, default_value = "10", value_parser = clap::value_parser!(i32).range(1..=1000))]
    pub limit: i32,

    #[arg(long, help = "Pagination cursor from previous response")]
    pub cursor: Option<String>,

    #[arg(long, value_parser = ["timestamp", "-timestamp"], help = SORT_HELP)]
    pub sort: Option<String>,
}

#[derive(Args)]
pub struct HostsArgs {
    #[arg(long, help = "Filter hosts by name, alias, or tag")]
    pub filter: Option<String>,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, help = "Sort field (e.g., cpu, iowait, load)")]
    pub sort_field: Option<String>,

    #[arg(long, help = "Sort direction (asc, desc)")]
    pub sort_dir: Option<String>,

    #[arg(long, default_value = "0", value_parser = clap::value_parser!(i32).range(0..), help = "Pagination offset")]
    pub start: i32,

    #[arg(long, default_value = "100", value_parser = clap::value_parser!(i32).range(1..=1000), help = "Results per page (max 1000)")]
    pub count: i32,

    #[arg(long, help = "Tag prefixes to include (default from config)")]
    pub tag_filter: Option<String>,
}

#[derive(Args)]
pub struct SpansArgs {
    #[arg(default_value = "*")]
    pub query: String,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,

    #[arg(long, default_value = "10", value_parser = clap::value_parser!(i32).range(1..=1000))]
    pub limit: i32,

    #[arg(long, help = "Pagination cursor from previous response")]
    pub cursor: Option<String>,

    #[arg(long, value_parser = ["timestamp", "-timestamp"], help = SORT_HELP)]
    pub sort: Option<String>,

    #[arg(long, help = "Tag prefixes to include (default from config)")]
    pub tag_filter: Option<String>,

    #[arg(long, help = "Show full stack traces")]
    pub full_stack_trace: bool,
}

#[derive(Args)]
pub struct ServicesArgs {
    #[arg(
        long,
        default_value = "service",
        help = "Entity kind (service, system, datastore, queue, api)"
    )]
    pub kind: String,

    #[arg(long, help = "Filter by entity name")]
    pub name: Option<String>,

    #[arg(long, help = "Filter by owner")]
    pub owner: Option<String>,

    #[arg(long, value_parser = ["schema", "raw_schema", "oncall", "incident", "relation"], help = "Include related data")]
    pub include: Option<String>,

    #[arg(long, default_value = "0", value_parser = clap::value_parser!(i32).range(0..), help = "Pagination offset")]
    pub start: i32,

    #[arg(long, default_value = "100", value_parser = clap::value_parser!(i32).range(1..), help = "Results per page")]
    pub count: i32,
}

#[derive(Args)]
pub struct RumArgs {
    #[arg(default_value = "*")]
    pub query: String,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,

    #[arg(long, default_value = "10", value_parser = clap::value_parser!(i32).range(1..=1000))]
    pub limit: i32,

    #[arg(long, help = "Pagination cursor from previous response")]
    pub cursor: Option<String>,

    #[arg(long, value_parser = ["timestamp", "-timestamp"], help = SORT_HELP)]
    pub sort: Option<String>,

    #[arg(long, help = "Tag prefixes to include (default from config)")]
    pub tag_filter: Option<String>,

    #[arg(long, help = "Show full stack traces")]
    pub full_stack_trace: bool,
}

#[derive(Subcommand)]
pub enum LogsAction {
    #[command(about = "Search logs")]
    Search(LogsSearchArgs),

    #[command(about = "Aggregate logs into buckets")]
    Aggregate(LogsAggregateArgs),

    #[command(about = "Generate log timeseries")]
    Timeseries(LogsTimeseriesArgs),
}

#[derive(Args)]
pub struct LogsSearchArgs {
    #[arg(default_value = "*")]
    pub query: String,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,

    #[arg(long, default_value = "10", value_parser = clap::value_parser!(i32).range(1..=1000))]
    pub limit: i32,

    #[arg(long, help = "Pagination cursor from previous response")]
    pub cursor: Option<String>,

    #[arg(long, value_parser = ["timestamp", "-timestamp"], help = SORT_HELP)]
    pub sort: Option<String>,

    #[arg(long, value_parser = ["indexes", "online-archives", "flex"], help = "Storage tier to search (default: indexes)")]
    pub storage_tier: Option<String>,

    #[arg(long, help = "Tag prefixes to include (default from config)")]
    pub tag_filter: Option<String>,
}

#[derive(Args)]
pub struct LogsGroupByArgs {
    #[arg(long, value_parser = clap::builder::NonEmptyStringValueParser::new(), help = "Group by a facet; buckets are ordered by descending log count")]
    pub group_by: Option<String>,

    #[arg(long, requires = "group_by", value_parser = clap::value_parser!(i32).range(1..=10000), help = "Maximum groups to return (default 10, max 10000)")]
    pub group_limit: Option<i32>,
}

#[derive(Args)]
pub struct LogsAggregateArgs {
    #[arg(default_value = "*")]
    pub query: String,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,

    #[command(flatten)]
    pub grouping: LogsGroupByArgs,
}

#[derive(Args)]
pub struct LogsTimeseriesArgs {
    #[arg(default_value = "*")]
    pub query: String,

    #[arg(long, default_value = "1 hour ago", help = TIME_HELP)]
    pub from: String,

    #[arg(long, default_value = "now", help = TIME_HELP)]
    pub to: String,

    #[arg(
        long,
        default_value = "1h",
        help = "Rollup interval (e.g., 5m, 1h, 1d)"
    )]
    pub interval: String,

    #[arg(
        long,
        default_value = "count",
        help = "Aggregation type (count, avg, sum, min, max)"
    )]
    pub aggregation: String,

    #[arg(long, help = "Metric field for aggregation")]
    pub metric: Option<String>,

    #[command(flatten)]
    pub grouping: LogsGroupByArgs,
}

#[derive(Subcommand)]
pub enum MonitorsAction {
    #[command(about = "List monitors")]
    List(MonitorsListArgs),

    #[command(about = "Get monitor details")]
    Get { monitor_id: i64 },
}

#[derive(Args)]
pub struct MonitorsListArgs {
    #[arg(long, help = "Filter by resource tags")]
    pub tags: Option<String>,

    #[arg(long, help = "Filter by monitor tags")]
    pub monitor_tags: Option<String>,

    #[arg(long, default_value = "0", value_parser = clap::value_parser!(i32).range(0..), help = "Page number")]
    pub page: i32,

    #[arg(long, default_value = "100", value_parser = clap::value_parser!(i32).range(1..=1000), help = "Results per page (max 1000)")]
    pub page_size: i32,
}

#[derive(Subcommand)]
pub enum DashboardsAction {
    #[command(about = "List dashboards")]
    List(DashboardsListArgs),

    #[command(about = "Get dashboard details")]
    Get { dashboard_id: String },
}

#[derive(Args)]
pub struct DashboardsListArgs {
    #[arg(long, default_value = "100", value_parser = clap::value_parser!(i32).range(1..), help = "Results per page")]
    pub count: i32,

    #[arg(long, default_value = "0", value_parser = clap::value_parser!(i32).range(0..), help = "Pagination offset")]
    pub start: i32,

    #[arg(long, help = "Include shared dashboards only")]
    pub filter_shared: bool,

    #[arg(long, help = "Include deleted dashboards only")]
    pub filter_deleted: bool,
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
    if let Command::Query(ref args) = cli.command {
        let mut input = crate::query::read_input(args.input.as_deref()).await?;
        let scope = crate::query::apply_time_range(
            &mut input,
            &args.domain,
            &args.operation,
            args.from.as_deref(),
            args.to.as_deref(),
            chrono::Utc::now(),
        )?;
        let prepared = crate::query::PreparedQuery::prepare(
            &args.domain,
            &args.operation,
            &input,
            args.output.clone(),
        )?
        .with_time_scope(scope)
        .with_selection(&args.select)?;
        let duration = std::time::Duration::from_secs(args.deadline);
        let (result, format) = tokio::time::timeout(duration, async {
            let started = std::time::Instant::now();
            let config = Config::load_with_options(cli.config_options()).await?;
            let client = DatadogClient::new(&config)?;
            let remaining = duration.saturating_sub(started.elapsed());
            let max_bytes = usize::try_from(args.max_bytes).map_err(|_| {
                crate::error::DatadogError::InvalidInput(
                    "Response byte budget is too large for this platform".into(),
                )
            })?;
            let result = prepared
                .execute(&client, &config.site, remaining, max_bytes, args.max_calls)
                .await?;
            let format =
                output::Format::from_str(cli.format.as_deref().unwrap_or(&config.defaults.format))
                    .map_err(crate::error::DatadogError::InvalidInput)?;
            Ok::<_, crate::error::DatadogError>((result, format))
        })
        .await
        .map_err(|_| crate::error::DatadogError::TimeoutError)??;
        output::print_api_diagnostics(&result)?;
        return output::print(&result, &format).map_err(Into::into);
    }
    if let Command::Capabilities { ref action } = cli.command {
        let data = discovery::execute(action)?;
        let format = output::Format::from_str(cli.format.as_deref().unwrap_or("json"))
            .map_err(crate::error::DatadogError::InvalidInput)?;
        return output::print(&data, &format).map_err(Into::into);
    }
    if let Command::Config { ref action } = cli.command {
        if !matches!(action, ConfigAction::Show)
            && (cli.auth_provider.is_some() || cli.auth_method.is_some() || cli.pup_org.is_some())
        {
            return Err(crate::error::DatadogError::InvalidInput(
                "Authentication options apply to API commands and config show".into(),
            ));
        }
        return commands::handle_config(action, &cli);
    }

    let config = Config::load_with_options(cli.config_options()).await?;
    let client = DatadogClient::new(&config)?;

    let format_str = cli.format.as_deref().unwrap_or(&config.defaults.format);
    let format =
        output::Format::from_str(format_str).map_err(crate::error::DatadogError::InvalidInput)?;

    let result = commands::execute(&cli.command, &client).await?;
    output::print_api_diagnostics(&result)?;
    if matches!(cli.command, Command::Scalar(_)) {
        output::print_scalar(&result, &format)?;
    } else {
        output::print(&result, &format)?;
    }

    Ok(())
}

impl Cli {
    fn config_options(&self) -> ConfigOptions {
        ConfigOptions {
            api_key: self.api_key.clone(),
            app_key: self.app_key.clone(),
            token: self.token.clone(),
            site: self.site.clone(),
            auth_method: self
                .auth_method
                .or(self.auth_provider.map(|_| AuthMethod::Oauth)),
            pup_org: self.pup_org.clone(),
        }
    }
}
