use serde_json::{json, Value};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ParameterParser, ResponseFormatter};

pub struct MonitorsHandler;

impl ResponseFormatter for MonitorsHandler {}
impl ParameterParser for MonitorsHandler {}

impl MonitorsHandler {
    pub async fn list(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = MonitorsHandler;

        let tags = handler.extract_string(params, "tags");
        let monitor_tags = handler.extract_string(params, "monitor_tags");
        let page = handler.extract_i32(params, "page", 0);
        let page_size = handler.extract_i32(params, "page_size", 100);

        let monitors = client
            .list_monitors(tags, monitor_tags, Some(page), Some(page_size))
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

        let pagination = PaginationInfo::single_page(data.len(), page_size as usize);

        Ok(handler.format_list(
            json!(data),
            Some(serde_json::to_value(pagination)?),
            None,
        ))
    }

    pub async fn get(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = MonitorsHandler;

        let monitor_id = params["monitor_id"]
            .as_i64()
            .ok_or_else(|| crate::error::DatadogError::InvalidInput("Missing monitor_id".into()))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_parser() {
        let handler = MonitorsHandler;
        let params = json!({
            "page": 1,
            "page_size": 50,
            "tags": "env:prod"
        });

        assert_eq!(handler.extract_i32(&params, "page", 0), 1);
        assert_eq!(handler.extract_i32(&params, "page_size", 100), 50);
        assert_eq!(handler.extract_string(&params, "tags"), Some("env:prod".to_string()));
    }
}
