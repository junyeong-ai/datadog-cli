use serde_json::{Value, json};

use crate::cli::DowntimesArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{ResponseFormatter, api_count};
use crate::pagination::PaginationInfo;

pub struct DowntimesHandler;

impl ResponseFormatter for DowntimesHandler {}

impl DowntimesHandler {
    pub async fn list(client: &DatadogClient, args: &DowntimesArgs) -> Result<Value> {
        let handler = DowntimesHandler;

        let response = client
            .list_downtimes(args.current_only, args.offset, args.limit)
            .await?;

        let data = json!(crate::datadog::models::response_records(&response)?);
        let returned = data.as_array().map(|a| a.len()).unwrap_or(0);

        let pagination = PaginationInfo::from_offset(
            returned,
            args.offset as usize,
            args.limit as usize,
            api_count(response.pointer("/meta/page/total_filtered_count"))?,
        );

        Ok(handler.with_api_meta(
            handler.format_list(data, Some(serde_json::to_value(pagination)?), None),
            response.get("meta").cloned(),
        ))
    }
}
