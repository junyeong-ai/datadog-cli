use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::ResponseFormatter;

pub struct MonitorsHandler;

impl ResponseFormatter for MonitorsHandler {}

impl MonitorsHandler {
    pub async fn list(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = MonitorsHandler;
        let tags = params["tags"].as_str().map(|s| s.to_string());
        let monitor_tags = params["monitor_tags"].as_str().map(|s| s.to_string());

        let monitors = client.list_monitors(tags, monitor_tags, None, None).await?;

        let data = json!(
            monitors
                .iter()
                .map(|monitor| {
                    json!({
                        "id": monitor.id,
                        "name": monitor.name,
                        "type": monitor.monitor_type,
                        "query": monitor.query,
                        "status": monitor.overall_state,
                        "tags": monitor.tags,
                        "priority": monitor.priority
                    })
                })
                .collect::<Vec<_>>()
        );

        Ok(handler.format_detail(data))
    }

    pub async fn get(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = MonitorsHandler;

        let monitor_id = params["monitor_id"].as_i64().ok_or_else(|| {
            crate::error::DatadogError::InvalidInput("Missing 'monitor_id' parameter".to_string())
        })?;

        let response = client.get_monitor(monitor_id).await?;

        let data = json!({
            "id": response.id,
            "name": response.name,
            "type": response.monitor_type,
            "query": response.query,
            "message": response.message,
            "tags": response.tags,
            "created": response.created,
            "modified": response.modified,
            "overall_state": response.overall_state,
            "priority": response.priority,
            "options": response.options.as_ref().map(|o| {
                let mut opts = json!({
                    "thresholds": o.thresholds,
                    "notify_no_data": o.notify_no_data,
                    "notify_audit": o.notify_audit,
                    "timeout_h": o.timeout_h
                });

                // Only include silenced if it has entries
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
    use serde_json::json;

    #[test]
    fn test_optional_tags_parameter() {
        let params_with = json!({"tags": "env:prod"});
        let params_without = json!({});

        assert_eq!(params_with["tags"].as_str(), Some("env:prod"));
        assert_eq!(params_without["tags"].as_str(), None);
    }

    #[test]
    fn test_optional_monitor_tags_parameter() {
        let params = json!({"monitor_tags": "service:web"});
        assert_eq!(params["monitor_tags"].as_str(), Some("service:web"));
    }

    #[test]
    fn test_get_missing_monitor_id() {
        let params = json!({});
        let monitor_id = params["monitor_id"].as_i64();
        assert_eq!(monitor_id, None);
    }

    #[test]
    fn test_get_valid_monitor_id() {
        let params = json!({"monitor_id": 12345});
        let monitor_id = params["monitor_id"].as_i64();
        assert_eq!(monitor_id, Some(12345));
    }
}
