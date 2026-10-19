use clap::{Args, Subcommand};

#[derive(Args)]
pub struct McpLimits {
    #[arg(long,default_value="60",value_parser=clap::value_parser!(u64).range(1..=3600),help="Authentication and MCP execution deadline in seconds")]
    pub deadline: u64,
    #[arg(long,default_value="8",value_parser=clap::value_parser!(u32).range(1..=1000),help="Maximum HTTP calls, including handshake, discovery and cleanup")]
    pub max_calls: u32,
    #[arg(long,default_value="8388608",value_parser=clap::value_parser!(u64).range(1..=536870912),help="Maximum response bytes across the MCP session")]
    pub max_bytes: u64,
}

#[derive(Subcommand)]
pub enum McpAction {
    #[command(about = "Discover reviewed MCP tools offline; server availability is not probed")]
    List {
        #[arg(long)]
        toolset: Option<String>,
        #[arg(
            long,
            help = "Literal case-insensitive text in tool names and descriptions"
        )]
        search: Option<String>,
        #[arg(long, default_value = "0")]
        offset: usize,
        #[arg(long,default_value="25",value_parser=clap::value_parser!(u32).range(1..=200))]
        limit: u32,
    },
    #[command(about = "Read the selected tool's current schema from the official MCP server")]
    Show {
        tool: String,
        #[command(flatten)]
        limits: McpLimits,
    },
    #[command(about = "Call a reviewed MCP tool after validating its current input schema")]
    Call {
        tool: String,
        #[arg(long, help = "JSON arguments, @FILE, or - for stdin (default {})")]
        input: Option<String>,
        #[command(flatten)]
        limits: McpLimits,
    },
}

pub async fn run(action: &McpAction, cli: &super::Cli) -> crate::Result<()> {
    use crate::{error::DatadogError, output};
    use std::time::{Duration, Instant};
    if let McpAction::List {
        toolset,
        search,
        offset,
        limit,
    } = action
    {
        let data = crate::mcp::list(
            toolset.as_deref(),
            search.as_deref(),
            *offset,
            *limit as usize,
        )?;
        let format = output::Format::from_str(cli.format.as_deref().unwrap_or("json"))
            .map_err(DatadogError::InvalidInput)?;
        return output::print(&data, &format).map_err(Into::into);
    }
    let (tool, limits) = match action {
        McpAction::Show { tool, limits } | McpAction::Call { tool, limits, .. } => (tool, limits),
        _ => unreachable!(),
    };
    let mut prepared = crate::mcp::PreparedMcp::prepare(tool, None)?;
    if let McpAction::Call { input, .. } = action {
        prepared = crate::mcp::PreparedMcp::prepare(
            tool,
            Some(crate::query::read_input(input.as_deref()).await?),
        )?;
    }
    let duration = Duration::from_secs(limits.deadline);
    let (data, format) = tokio::time::timeout(duration, async {
        let started = Instant::now();
        let config = crate::Config::load_with_site_check(cli.config_options(), |site| {
            prepared.check_site(site)
        })
        .await?;
        let max_bytes = usize::try_from(limits.max_bytes).map_err(|_| {
            DatadogError::InvalidInput("Response byte budget is too large for this platform".into())
        })?;
        let data = prepared
            .execute(
                &config,
                duration.saturating_sub(started.elapsed()),
                limits.max_calls,
                max_bytes,
            )
            .await?;
        let format =
            output::Format::from_str(cli.format.as_deref().unwrap_or(&config.defaults.format))
                .map_err(DatadogError::InvalidInput)?;
        Ok::<_, DatadogError>((data, format))
    })
    .await
    .map_err(|_| DatadogError::TimeoutError)??;
    output::print(&data, &format).map_err(Into::into)
}
