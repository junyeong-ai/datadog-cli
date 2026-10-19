pub mod client;
pub mod models;
mod retry;
mod transport;

pub use client::{LlmObsFilter, LlmObsSearchParams, MAX_METRIC_QUERIES, SearchParams};
pub use transport::DatadogClient;
pub(crate) use transport::{ApiReply, QueryBudget, QueryCall, query_headers};

pub(crate) fn validate_query_processing(
    operation: &str,
    body: &serde_json::Value,
) -> crate::error::Result<()> {
    use models::{FormulaResponse, MetricsResponse, ScalarAttributes, TimeseriesAttributes};
    match operation {
        "v1.QueryMetrics" => {
            MetricsResponse::from_api_response(body.clone())?;
        }
        "v2.QueryTimeseriesData" => {
            serde_json::from_value::<FormulaResponse>(body.clone())
                .map_err(|error| crate::error::DatadogError::DecodeError(error.to_string()))?
                .into_attributes::<TimeseriesAttributes>()?
                .validate()?;
        }
        "v2.QueryScalarData" => {
            serde_json::from_value::<FormulaResponse>(body.clone())
                .map_err(|error| crate::error::DatadogError::DecodeError(error.to_string()))?
                .into_attributes::<ScalarAttributes>()?
                .validate()?;
        }
        "v2.SearchLLMObsSpans" | "v2.ListLLMObsSpans" => {
            models::validate_llm_response(body.clone())?;
        }
        "v2.ListIncidents" => {
            models::validate_required_records(body.clone())?;
        }
        "v2.GetIncident" => {
            models::validate_required_resource(body.clone())?;
        }
        _ => {}
    }
    Ok(())
}
