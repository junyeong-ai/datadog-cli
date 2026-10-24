use clap::Subcommand;
use serde_json::Value;

use crate::catalog::Catalog;
use crate::error::Result;

#[derive(Subcommand)]
pub enum CapabilitiesAction {
    #[command(about = "List API domains; organization access is not probed")]
    Domains {
        #[arg(long, help = "Include domains with only excluded operations")]
        all: bool,
    },
    #[command(about = "List reviewed API operations")]
    List {
        #[arg(long, help = "Exact domain ID from capabilities domains")]
        domain: Option<String>,
        #[arg(long, help = "Include excluded operations and their reasons")]
        all: bool,
        #[arg(
            long,
            help = "Literal case-insensitive text in the operation ID or summary"
        )]
        search: Option<String>,
        #[arg(long, default_value = "0", help = "Pagination offset")]
        offset: usize,
        #[arg(long, default_value="25", value_parser=clap::value_parser!(u32).range(1..=200), help = "Results per page (max 200)")]
        limit: u32,
    },
    #[command(about = "Describe an operation, including its offline JSON input schema")]
    Show {
        #[arg(help = "Version-qualified operation ID, for example v2.SearchCIAppPipelineEvents")]
        operation: String,
        #[arg(
            long,
            help = "Include complete response schemas; large products can have large schemas"
        )]
        response_schema: bool,
    },
}

pub fn execute(action: &CapabilitiesAction) -> Result<Value> {
    let catalog = Catalog::load()?;
    match action {
        CapabilitiesAction::Domains { all } => Ok(catalog.domains(*all)),
        CapabilitiesAction::List {
            domain,
            all,
            search,
            offset,
            limit,
        } => catalog.list(
            domain.as_deref(),
            *all,
            search.as_deref(),
            *offset,
            *limit as usize,
        ),
        CapabilitiesAction::Show {
            operation,
            response_schema,
        } => catalog.describe(operation, *response_schema),
    }
}
