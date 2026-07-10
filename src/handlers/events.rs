use serde_json::{Value, json};

use crate::cli::EventsArgs;
use crate::datadog::{DatadogClient, SearchParams};
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter, TimeHandler};

pub struct EventsHandler;

impl TimeHandler for EventsHandler {}
impl ResponseFormatter for EventsHandler {}

impl EventsHandler {
    pub async fn search(client: &DatadogClient, args: &EventsArgs) -> Result<Value> {
        let handler = EventsHandler;

        let (from_iso, to_iso) = handler.parse_time_range_iso8601(&args.from, &args.to)?;

        let response = client
            .search_events(&SearchParams {
                query: &args.query,
                from: &from_iso,
                to: &to_iso,
                limit: args.limit,
                cursor: args.cursor.as_deref(),
                sort: args.sort.as_deref(),
            })
            .await?;

        let events: Vec<Value> = response
            .data
            .unwrap_or_default()
            .iter()
            .map(|event| {
                let mut entry = json!({ "id": event.id });

                if let Some(event_type) = &event.event_type {
                    entry["type"] = json!(event_type);
                }

                if let Some(attrs) = &event.attributes {
                    for key in ["timestamp", "message", "tags", "attributes"] {
                        if let Some(value) = attrs.get(key)
                            && !value.is_null()
                        {
                            entry[key] = value.clone();
                        }
                    }
                }

                entry
            })
            .collect();

        let next_cursor = response
            .meta
            .as_ref()
            .and_then(|m| m.page.as_ref())
            .and_then(|p| p.after.clone());

        let pagination =
            PaginationInfo::from_cursor(events.len(), args.limit as usize, next_cursor);

        Ok(json!({
            "data": events,
            "pagination": pagination
        }))
    }
}
