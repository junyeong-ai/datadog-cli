use serde_json::{Value, json};

use crate::cli::LlmObsArgs;
use crate::datadog::{DatadogClient, LlmObsFilter, LlmObsSearchParams};
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter, TimeHandler};

pub struct LlmObsHandler;

impl TimeHandler for LlmObsHandler {}
impl ResponseFormatter for LlmObsHandler {}

impl LlmObsHandler {
    pub async fn search(client: &DatadogClient, args: &LlmObsArgs) -> Result<Value> {
        let handler = LlmObsHandler;

        let (from_iso, to_iso) = handler.parse_time_range_iso8601(&args.from, &args.to)?;

        let response = client
            .search_llm_obs_spans(&LlmObsSearchParams {
                filter: match args.query.as_deref() {
                    Some(query) => LlmObsFilter::Query(query),
                    None => LlmObsFilter::Structured {
                        ml_app: args.ml_app.as_deref(),
                        span_kind: args.span_kind.as_deref(),
                    },
                },
                from: &from_iso,
                to: &to_iso,
                limit: args.limit,
                cursor: args.cursor.as_deref(),
                sort: args.sort.as_deref(),
            })
            .await?;

        let data = json!(crate::datadog::models::response_records(&response)?);
        let returned = data.as_array().map(|a| a.len()).unwrap_or(0);

        let next_cursor = response
            .get("meta")
            .and_then(|m| m.get("page"))
            .and_then(|p| p.get("after"))
            .and_then(|a| a.as_str())
            .map(String::from);

        let pagination = PaginationInfo::from_cursor(returned, args.limit as usize, next_cursor);

        Ok(handler.with_api_meta(
            json!({
                "data": data,
                "pagination": pagination
            }),
            response.get("meta").cloned(),
        ))
    }
}
