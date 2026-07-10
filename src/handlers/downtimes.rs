use serde_json::{Value, json};

use crate::cli::DowntimesArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter};

pub struct DowntimesHandler;

impl ResponseFormatter for DowntimesHandler {}

impl DowntimesHandler {
    pub async fn list(client: &DatadogClient, args: &DowntimesArgs) -> Result<Value> {
        let handler = DowntimesHandler;

        let response = client
            .list_downtimes(args.current_only, args.start, args.count)
            .await?;

        let data = response.get("data").cloned().unwrap_or_else(|| json!([]));
        let returned = data.as_array().map(|a| a.len()).unwrap_or(0);

        let pagination = PaginationInfo::from_offset_without_total(
            returned,
            args.start as usize,
            args.count as usize,
        );

        Ok(handler.format_list(data, Some(serde_json::to_value(pagination)?), None))
    }
}
