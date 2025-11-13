use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{ResponseFormatter, TimeHandler, TimeParams};

pub struct EventsHandler;

impl TimeHandler for EventsHandler {}
impl ResponseFormatter for EventsHandler {}

impl EventsHandler {
    pub async fn query(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = EventsHandler;

        let priority = params["priority"].as_str().map(|s| s.to_string());
        let sources = params["sources"].as_str().map(|s| s.to_string());
        let tags = params["tags"].as_str().map(|s| s.to_string());

        let time = handler.parse_time(params, 1)?;

        let TimeParams::Timestamp {
            from: start,
            to: end,
        } = time;

        let response = client
            .query_events(start, end, priority, sources, tags)
            .await?;

        let events = response.events.unwrap_or_default();

        let data = json!(
            events
                .iter()
                .map(|event| {
                    json!({
                        "id": event.id,
                        "title": event.title,
                        "text": event.text,
                        "date": event.date_happened.map(crate::utils::format_timestamp),
                        "priority": event.priority,
                        "host": event.host,
                        "source": event.source,
                        "alert_type": event.alert_type,
                        "tags": event.tags
                    })
                })
                .collect::<Vec<_>>()
        );

        Ok(handler.format_detail(data))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_optional_parameters() {
        let params_full = json!({
            "priority": "normal",
            "sources": "my_apps",
            "tags": "env:prod"
        });

        assert_eq!(params_full["priority"].as_str(), Some("normal"));
        assert_eq!(params_full["sources"].as_str(), Some("my_apps"));
        assert_eq!(params_full["tags"].as_str(), Some("env:prod"));
    }

    #[test]
    fn test_missing_optional_parameters() {
        let params_empty = json!({});

        assert_eq!(params_empty["priority"].as_str(), None);
        assert_eq!(params_empty["sources"].as_str(), None);
        assert_eq!(params_empty["tags"].as_str(), None);
    }
}
