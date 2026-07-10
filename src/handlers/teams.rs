use serde_json::{Value, json};

use crate::cli::TeamsArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter};

pub struct TeamsHandler;

impl ResponseFormatter for TeamsHandler {}

impl TeamsHandler {
    pub async fn list(client: &DatadogClient, args: &TeamsArgs) -> Result<Value> {
        let handler = TeamsHandler;

        let response = client
            .list_teams(args.keyword.as_deref(), args.me, args.page, args.page_size)
            .await?;

        let data = response.get("data").cloned().unwrap_or_else(|| json!([]));
        let returned = data.as_array().map(|a| a.len()).unwrap_or(0);

        let pagination =
            PaginationInfo::from_page_number(returned, args.page as usize, args.page_size as usize);

        Ok(handler.format_list(data, Some(serde_json::to_value(pagination)?), None))
    }
}
