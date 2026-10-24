use serde_json::{Value, json};

use crate::cli::MonitorsListArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::ResponseFormatter;
use crate::pagination::PaginationInfo;

pub struct MonitorsHandler;

impl ResponseFormatter for MonitorsHandler {}

impl MonitorsHandler {
    pub async fn list(client: &DatadogClient, args: &MonitorsListArgs) -> Result<Value> {
        let handler = MonitorsHandler;

        let monitors = client
            .list_monitors(
                args.tags.as_deref(),
                args.monitor_tags.as_deref(),
                Some(args.page),
                Some(args.limit),
            )
            .await?;

        let data: Vec<Value> = monitors
            .iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "name": m.name,
                    "type": m.monitor_type,
                    "overall_state": m.overall_state,
                    "tags": m.tags,
                    "priority": m.priority,
                    "created": m.created,
                    "modified": m.modified,
                })
            })
            .collect();

        let pagination =
            PaginationInfo::from_page(data.len(), args.page as usize, args.limit as usize, None);

        Ok(handler.format_list(json!(data), Some(serde_json::to_value(pagination)?), None))
    }

    pub async fn get(client: &DatadogClient, monitor_id: i64) -> Result<Value> {
        let handler = MonitorsHandler;

        let m = client.get_monitor(monitor_id).await?;

        let data = serde_json::to_value(m)?;

        Ok(handler.format_detail(data))
    }
}
