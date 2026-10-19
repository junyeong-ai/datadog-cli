use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{Value, json};
use tokio::time::Instant;

use super::{DatadogClient, retry};
use crate::catalog::RetryPolicy;
use crate::error::{DatadogError, Result};

pub(crate) struct QueryCall {
    pub method: reqwest::Method,
    pub endpoint: String,
    pub parameters: Vec<(String, String)>,
    pub headers: HeaderMap,
    pub body: Option<Value>,
    pub content_type: String,
    pub accept: String,
    pub retry: RetryPolicy,
    pub accepted_statuses: Vec<u16>,
}

pub(crate) struct ApiReply {
    pub status: u16,
    pub headers: Value,
    pub bytes: Vec<u8>,
}

pub(crate) struct QueryBudget {
    pub deadline: Instant,
    pub max_bytes: usize,
    pub max_calls: u32,
    pub calls: u32,
    pub received_bytes: usize,
}

impl QueryBudget {
    pub fn new(duration: Duration, max_bytes: usize, max_calls: u32) -> Self {
        Self {
            deadline: Instant::now() + duration,
            max_bytes,
            max_calls,
            calls: 0,
            received_bytes: 0,
        }
    }
}

impl DatadogClient {
    pub(crate) async fn execute_query(
        &self,
        call: &QueryCall,
        budget: &mut QueryBudget,
    ) -> Result<ApiReply> {
        let now = Instant::now();
        let remaining = budget.deadline.saturating_duration_since(now);
        let deadline = (now + self.total_timeout.min(remaining)).min(budget.deadline);
        tokio::time::timeout_at(deadline, self.execute_query_attempts(call, budget))
            .await
            .map_err(|_| DatadogError::TimeoutError)?
    }

    async fn execute_query_attempts(
        &self,
        call: &QueryCall,
        budget: &mut QueryBudget,
    ) -> Result<ApiReply> {
        let mut attempt = 0;
        loop {
            if budget.calls >= budget.max_calls {
                return Err(DatadogError::QueryError("API call budget exhausted".into()));
            }
            budget.calls += 1;
            let mut request = self
                .client
                .request(
                    call.method.clone(),
                    format!("{}{}", self.base_url, call.endpoint),
                )
                .headers(call.headers.clone())
                .header("accept", &call.accept)
                .query(&call.parameters);
            if let Some(body) = &call.body {
                request = request
                    .header("content-type", &call.content_type)
                    .json(body);
            }
            let mut retry_after = None;
            let result = match request.send().await {
                Err(error) => Err(network_error(error)),
                Ok(mut response) => {
                    let status = response.status();
                    let headers = response.headers().clone();
                    retry_after = retry::retry_after(&headers);
                    match read_bytes(&mut response, budget).await {
                        Err(error) => Err(error),
                        Ok(bytes) if call.accepted_statuses.contains(&status.as_u16()) => {
                            Ok(ApiReply {
                                status: status.as_u16(),
                                headers: response_headers(&headers),
                                bytes,
                            })
                        }
                        Ok(bytes) => {
                            let message = String::from_utf8_lossy(&bytes).into_owned();
                            Err(match status.as_u16() {
                                401 => DatadogError::AuthError(message),
                                403 => DatadogError::PermissionDenied(message),
                                429 => DatadogError::RateLimitError {
                                    reset_secs: headers
                                        .get("x-ratelimit-reset")
                                        .and_then(|value| value.to_str().ok())
                                        .and_then(|value| value.parse().ok())
                                        .into_iter()
                                        .chain(retry_after)
                                        .max(),
                                },
                                status => DatadogError::ApiError { status, message },
                            })
                        }
                    }
                }
            };
            match result {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if call.retry == RetryPolicy::Never || budget.calls >= budget.max_calls {
                        return Err(error);
                    }
                    match retry::next_delay(&error, attempt, self.max_retries, retry_after) {
                        Some(delay) => {
                            attempt += 1;
                            tokio::time::sleep(delay).await;
                        }
                        None => return Err(error),
                    }
                }
            }
        }
    }
}

async fn read_bytes(response: &mut reqwest::Response, budget: &mut QueryBudget) -> Result<Vec<u8>> {
    if response.content_length().is_some_and(|length| {
        length > budget.max_bytes.saturating_sub(budget.received_bytes) as u64
    }) {
        return Err(DatadogError::QueryError(format!(
            "Response exceeds the {} byte execution budget",
            budget.max_bytes
        )));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(network_error)? {
        if chunk.len() > budget.max_bytes.saturating_sub(budget.received_bytes) {
            return Err(DatadogError::QueryError(format!(
                "Response exceeds the {} byte execution budget",
                budget.max_bytes
            )));
        }
        budget.received_bytes += chunk.len();
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn network_error(error: reqwest::Error) -> DatadogError {
    if error.is_timeout() {
        DatadogError::TimeoutError
    } else if error.is_builder() {
        DatadogError::InvalidInput("Cannot build API request".into())
    } else {
        DatadogError::NetworkError(error.without_url())
    }
}

fn response_headers(headers: &HeaderMap) -> Value {
    let mut result = serde_json::Map::new();
    for name in [
        "content-type",
        "etag",
        "x-datadog-request-id",
        "x-request-id",
        "x-ratelimit-limit",
        "x-ratelimit-period",
        "x-ratelimit-remaining",
        "x-ratelimit-reset",
    ] {
        if let Some(value) = headers.get(name).and_then(|value| value.to_str().ok()) {
            result.insert(name.into(), json!(value));
        }
    }
    Value::Object(result)
}

pub(crate) fn query_headers(values: impl Iterator<Item = (String, String)>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    for (name, value) in values {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
            DatadogError::InvalidInput("Invalid declared request header name".into())
        })?;
        if [
            "authorization",
            "dd-api-key",
            "dd-application-key",
            "host",
            "content-length",
            "content-type",
            "accept",
        ]
        .contains(&name.as_str())
        {
            return Err(DatadogError::InvalidInput(
                "Credentials and transport headers cannot be overridden".into(),
            ));
        }
        let mut value = HeaderValue::from_str(&value).map_err(|_| {
            DatadogError::InvalidInput("Invalid declared request header value".into())
        })?;
        value.set_sensitive(true);
        headers.insert(name, value);
    }
    Ok(headers)
}
