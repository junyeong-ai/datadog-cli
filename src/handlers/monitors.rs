use serde_json::{Value, json};

use crate::cli::MonitorsListArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter};

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
                Some(args.page_size),
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

        let pagination = PaginationInfo::from_page_number(
            data.len(),
            args.page as usize,
            args.page_size as usize,
        );

        Ok(handler.format_list(json!(data), Some(serde_json::to_value(pagination)?), None))
    }

    pub async fn get(client: &DatadogClient, monitor_id: i64) -> Result<Value> {
        let handler = MonitorsHandler;

        let m = client.get_monitor(monitor_id).await?;

        let data = json!({
            "id": m.id,
            "name": m.name,
            "type": m.monitor_type,
            "query": m.query,
            "message": m.message,
            "tags": m.tags,
            "created": m.created,
            "modified": m.modified,
            "overall_state": m.overall_state,
            "priority": m.priority,
            "options": m.options.as_ref().map(|o| {
                let mut opts = json!({
                    "thresholds": o.thresholds,
                    "notify_no_data": o.notify_no_data,
                    "notify_audit": o.notify_audit,
                    "timeout_h": o.timeout_h
                });

                if let Some(ref silenced) = o.silenced
                    && let Some(obj) = silenced.as_object()
                    && !obj.is_empty()
                {
                    opts["silenced"] = json!(silenced);
                }

                opts
            })
        });

        Ok(handler.format_detail(data))
    }
}
