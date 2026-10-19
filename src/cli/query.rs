use std::path::PathBuf;

use clap::Args;

#[derive(Args)]
pub struct QueryArgs {
    #[arg(help = "Exact domain ID from capabilities domains")]
    pub domain: String,
    #[arg(help = "Version-qualified operation ID from capabilities list")]
    pub operation: String,
    #[arg(
        long,
        help = "JSON request envelope, @FILE, or - for stdin; keys: path, query, headers, body"
    )]
    pub input: Option<String>,
    #[arg(
        long,
        help = "Start time using this operation's reviewed time mapping; conflicts with native input time fields"
    )]
    pub from: Option<String>,
    #[arg(
        long,
        requires = "from",
        help = "End time for --from (default now); resolved with the same reference instant"
    )]
    pub to: Option<String>,
    #[arg(
        long,
        conflicts_with = "output",
        help = "Select an exact JSON Pointer from each API response; repeat for multiple fields"
    )]
    pub select: Vec<String>,
    #[arg(
        long,
        help = "Save the original API response to a new file; required for binary/CSV responses"
    )]
    pub output: Option<PathBuf>,
    #[arg(long, default_value = "60", value_parser=clap::value_parser!(u64).range(1..=3600), help = "Authentication and API execution deadline in seconds")]
    pub deadline: u64,
    #[arg(long, default_value = "4", value_parser=clap::value_parser!(u32).range(1..=1000), help = "Maximum API attempts, including retries")]
    pub max_calls: u32,
    #[arg(long, default_value = "8388608", value_parser=clap::value_parser!(u64).range(1..=536870912), help = "Maximum response bytes across API attempts")]
    pub max_bytes: u64,
}
