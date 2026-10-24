use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::cli::ErrorTrackingSearchArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::ResponseFormatter;

pub struct ErrorTrackingHandler;

impl ResponseFormatter for ErrorTrackingHandler {}

impl ErrorTrackingHandler {
    pub async fn search(
        client: &DatadogClient,
        args: &ErrorTrackingSearchArgs,
        now: DateTime<Utc>,
    ) -> Result<Value> {
        let handler = ErrorTrackingHandler;

        let range = args.range.resolve(now)?;

        let response = client
            .search_error_issues(
                &args.query,
                &args.track,
                range.from * 1000,
                range.to * 1000,
                args.include.as_deref(),
            )
            .await?;

        let data = json!(crate::datadog::models::response_records(&response)?);

        let mut meta = serde_json::Map::new();
        meta.insert("track".to_string(), json!(args.track));
        if let Some(included) = response.get("included").filter(|v| !v.is_null()) {
            meta.insert("included".to_string(), included.clone());
        }

        // This endpoint has no pagination: the result set is single-shot.
        Ok(handler.format_list(data, None, Some(Value::Object(meta))))
    }

    pub async fn get(client: &DatadogClient, issue_id: &str) -> Result<Value> {
        let handler = ErrorTrackingHandler;

        let response = client.get_error_issue(issue_id).await?;
        let data = crate::datadog::models::response_resource(&response)?;

        Ok(handler.format_detail(data))
    }
}
