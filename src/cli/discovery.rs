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
        CapabilitiesAction::List { domain, all } => catalog.list(domain.as_deref(), *all),
        CapabilitiesAction::Show {
            operation,
            response_schema,
        } => catalog.describe(operation, *response_schema),
    }
}
