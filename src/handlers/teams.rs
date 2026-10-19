use serde_json::{Value, json};

use crate::cli::TeamsArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter, api_count};

pub struct TeamsHandler;

impl ResponseFormatter for TeamsHandler {}

impl TeamsHandler {
    pub async fn list(client: &DatadogClient, args: &TeamsArgs) -> Result<Value> {
        let handler = TeamsHandler;

        let response = client
            .list_teams(args.keyword.as_deref(), args.me, args.page, args.page_size)
            .await?;

        let data = json!(crate::datadog::models::response_records(&response)?);
        let returned = data.as_array().map(|a| a.len()).unwrap_or(0);

        let total = api_count(response.pointer("/meta/pagination/total"))?;
        let mut pagination = PaginationInfo::from_page_number_total(
            returned,
            args.page as usize,
            args.page_size as usize,
            total,
        );
        if let Some(next) = response.pointer("/meta/pagination/next_number") {
            pagination.has_next = api_count(Some(next))?.is_some();
            pagination.has_next_is_estimate = false;
        }

        Ok(handler.with_api_meta(
            handler.format_list(data, Some(serde_json::to_value(pagination)?), None),
            response.get("meta").cloned(),
        ))
    }
}
