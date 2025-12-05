use serde_json::{json, Value};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ParameterParser, ResponseFormatter, TimeHandler, TimeParams};

pub struct EventsHandler;

impl TimeHandler for EventsHandler {}
impl ResponseFormatter for EventsHandler {}
impl ParameterParser for EventsHandler {}

impl EventsHandler {
    pub async fn query(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = EventsHandler;

        let priority = handler.extract_string(params, "priority");
        let sources = handler.extract_string(params, "sources");
        let tags = handler.extract_string(params, "tags");

        let time = handler.parse_time(params, 1)?;
        let TimeParams::Timestamp { from: start, to: end } = time;

        let response = client
            .query_events(start, end, priority, sources, tags)
            .await?;

        let events = response.events.unwrap_or_default();

        let data: Vec<Value> = events
            .iter()
            .map(|e| {
                json!({
                    "id": e.id,
                    "title": e.title,
                    "text": e.text,
                    "date": e.date_happened.map(crate::utils::format_timestamp),
                    "priority": e.priority,
                    "host": e.host,
                    "source": e.source,
                    "alert_type": e.alert_type,
                    "tags": e.tags,
                })
            })
            .collect();

        let pagination = PaginationInfo::single_page(data.len(), 1000);

        Ok(handler.format_list(
            json!(data),
            Some(serde_json::to_value(pagination)?),
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_parser() {
        let handler = EventsHandler;
        let params = json!({
            "priority": "normal",
            "sources": "my_apps",
            "tags": "env:prod",
        });

        assert_eq!(handler.extract_string(&params, "priority"), Some("normal".to_string()));
        assert_eq!(handler.extract_string(&params, "sources"), Some("my_apps".to_string()));
        assert_eq!(handler.extract_string(&params, "tags"), Some("env:prod".to_string()));
    }
}
