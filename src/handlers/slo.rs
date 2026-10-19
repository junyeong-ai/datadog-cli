use serde_json::{Value, json};

use crate::cli::SloListArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter, api_count};

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

        let data = json!(crate::datadog::models::response_records(&response)?);
        let returned = data.as_array().map(|a| a.len()).unwrap_or(0);

        let pagination = PaginationInfo::from_offset_total(
            returned,
            args.offset as usize,
            args.limit as usize,
            api_count(response.pointer("/metadata/page/total_filtered_count"))?,
        );

        Ok(handler.with_api_meta(
            handler.format_list(data, Some(serde_json::to_value(pagination)?), None),
            response.get("metadata").cloned(),
        ))
    }

    pub async fn get(client: &DatadogClient, slo_id: &str) -> Result<Value> {
        let handler = SloHandler;

        let response = client.get_slo(slo_id).await?;
        let data = crate::datadog::models::response_resource(&response)?;

        Ok(handler.format_detail(data))
    }
}
