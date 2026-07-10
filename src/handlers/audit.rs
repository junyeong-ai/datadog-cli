use serde_json::{Value, json};

use crate::cli::AuditArgs;
use crate::datadog::{DatadogClient, SearchParams};
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter, TimeHandler};

pub struct AuditHandler;

impl TimeHandler for AuditHandler {}
impl ResponseFormatter for AuditHandler {}

impl AuditHandler {
    pub async fn search(client: &DatadogClient, args: &AuditArgs) -> Result<Value> {
        let handler = AuditHandler;

        let (from_iso, to_iso) = handler.parse_time_range_iso8601(&args.from, &args.to)?;

        let response = client
            .search_audit_events(&SearchParams {
                query: &args.query,
                from: &from_iso,
                to: &to_iso,
                limit: args.limit,
                cursor: args.cursor.as_deref(),
                sort: args.sort.as_deref(),
            })
            .await?;

        let data = response.get("data").cloned().unwrap_or_else(|| json!([]));
        let returned = data.as_array().map(|a| a.len()).unwrap_or(0);

        let next_cursor = response
            .get("meta")
            .and_then(|m| m.get("page"))
            .and_then(|p| p.get("after"))
            .and_then(|a| a.as_str())
            .map(String::from);

        let pagination = PaginationInfo::from_cursor(returned, args.limit as usize, next_cursor);

        Ok(json!({
            "data": data,
            "pagination": pagination
        }))
    }
}
