use serde_json::{Value, json};

use crate::cli::{IncidentsGetArgs, IncidentsListArgs};
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter};

pub struct IncidentsHandler;

impl ResponseFormatter for IncidentsHandler {}

impl IncidentsHandler {
    pub async fn list(client: &DatadogClient, args: &IncidentsListArgs) -> Result<Value> {
        let handler = IncidentsHandler;

        let response = client
            .list_incidents(args.count, args.start, args.include.as_deref())
            .await?;

        let data = response.get("data").cloned().unwrap_or_else(|| json!([]));
        let returned = data.as_array().map(|a| a.len()).unwrap_or(0);

        let pagination = PaginationInfo::from_offset_without_total(
            returned,
            args.start as usize,
            args.count as usize,
        );

        let meta = response
            .get("included")
            .filter(|v| !v.is_null())
            .map(|included| json!({ "included": included }));

        Ok(handler.format_list(data, Some(serde_json::to_value(pagination)?), meta))
    }

    pub async fn get(client: &DatadogClient, args: &IncidentsGetArgs) -> Result<Value> {
        let handler = IncidentsHandler;

        let response = client
            .get_incident(&args.incident_id, args.include.as_deref())
            .await?;
        let data = response.get("data").cloned().unwrap_or(Value::Null);

        Ok(handler.format_detail(data))
    }
}
