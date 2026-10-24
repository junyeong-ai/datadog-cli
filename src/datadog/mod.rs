pub mod client;
pub mod models;
mod retry;
mod transport;

pub use client::{LlmObsFilter, LlmObsSearchParams, MAX_METRIC_QUERIES, SearchParams};
pub use transport::DatadogClient;
pub(crate) use transport::{ApiReply, QueryBudget, QueryCall, auth_headers, query_headers};

/// Rejects failures that metric queries report inside successful responses.
/// Other response fields reach `query` callers unchanged; their deviations
/// from the contract are reported, not enforced.
pub(crate) fn validate_query_processing(
    operation: &str,
    body: &serde_json::Value,
) -> crate::error::Result<()> {
    let error = match operation {
        "v1.QueryMetrics" => models::metric_query_error(body),
        "v2.QueryTimeseriesData" | "v2.QueryScalarData" => models::formula_query_error(body),
        _ => None,
    };
    error.map_or(Ok(()), |message| {
        Err(crate::error::DatadogError::QueryError(message))
    })
}
