use serde_json::{Value, json};

use crate::cli::SloListArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter};

pub struct SloHandler;

impl ResponseFormatter for SloHandler {}

impl SloHandler {
    pub async fn list(client: &DatadogClient, args: &SloListArgs) -> Result<Value> {
        let handler = SloHandler;

        let response = client
            .list_slos(
                args.query.as_deref(),
                args.tags_query.as_deref(),
                args.limit,
                args.offset,
            )
            .await?;

        let data = response.get("data").cloned().unwrap_or_else(|| json!([]));
        let returned = data.as_array().map(|a| a.len()).unwrap_or(0);

        // v1 SLO list reports the filtered total at metadata.page.total_filtered_count.
        let total_filtered = response
            .get("metadata")
            .and_then(|m| m.get("page"))
            .and_then(|p| p.get("total_filtered_count"))
            .and_then(|c| c.as_u64());

        let pagination = match total_filtered {
            Some(total) => PaginationInfo::from_offset(
                total as usize,
                args.offset as usize,
                args.limit as usize,
            ),
            None => PaginationInfo::from_offset_without_total(
                returned,
                args.offset as usize,
                args.limit as usize,
            ),
        };

        Ok(handler.format_list(data, Some(serde_json::to_value(pagination)?), None))
    }

    pub async fn get(client: &DatadogClient, slo_id: &str) -> Result<Value> {
        let handler = SloHandler;

        let response = client.get_slo(slo_id).await?;
        let data = response.get("data").cloned().unwrap_or(Value::Null);

        Ok(handler.format_detail(data))
    }
}
