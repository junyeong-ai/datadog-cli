use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use futures_util::{StreamExt, stream::BoxStream};
use reqwest::{
    Method,
    header::{HeaderName, HeaderValue},
};
use rmcp::{
    model::ClientJsonRpcMessage,
    transport::{
        common::client_side_sse::SseRetryPolicy,
        streamable_http_client::{
            StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
        },
    },
};
use sse_stream::{Sse, SseStream};

use crate::{
    config::Config,
    error::{DatadogError, Result},
};

type HttpError = StreamableHttpError<io::Error>;

#[derive(Debug)]
pub(super) struct NoReconnect;
impl SseRetryPolicy for NoReconnect {
    fn retry(&self, _: usize) -> Option<Duration> {
        None
    }
}

struct Usage {
    calls: u32,
    bytes: usize,
    failure: Option<DatadogError>,
}

#[derive(Clone)]
pub(super) struct BoundedHttp {
    client: reqwest::Client,
    uri: Arc<str>,
    state: Arc<Mutex<Usage>>,
    max_calls: u32,
    max_bytes: usize,
    deadline: Instant,
}

impl BoundedHttp {
    pub fn new(
        config: &Config,
        uri: String,
        duration: Duration,
        max_calls: u32,
        max_bytes: usize,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .default_headers(crate::datadog::auth_headers(&config.credentials)?)
            .timeout(duration.min(Duration::from_secs(config.network.timeout_secs)))
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .user_agent(concat!("datadog-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(DatadogError::NetworkError)?;
        Ok(Self {
            client,
            uri: uri.into(),
            state: Arc::new(Mutex::new(Usage {
                calls: 0,
                bytes: 0,
                failure: None,
            })),
            max_calls,
            max_bytes,
            deadline: Instant::now() + duration,
        })
    }

    fn fail(&self, error: DatadogError) -> HttpError {
        if let Ok(mut state) = self.state.lock()
            && state.failure.is_none()
        {
            state.failure = Some(error);
        }
        StreamableHttpError::Client(io::Error::other("MCP HTTP request failed"))
    }

    pub fn error(&self, fallback: DatadogError) -> DatadogError {
        self.state
            .lock()
            .ok()
            .and_then(|mut state| state.failure.take())
            .unwrap_or(fallback)
    }

    pub fn take_failure(&self) -> Option<DatadogError> {
        self.state
            .lock()
            .ok()
            .and_then(|mut state| state.failure.take())
    }

    pub fn usage(&self) -> Result<(u32, usize)> {
        let state = self
            .state
            .lock()
            .map_err(|_| DatadogError::QueryError("MCP execution state unavailable".into()))?;
        Ok((state.calls, state.bytes))
    }

    fn charge_bytes(&self, bytes: usize) -> std::result::Result<(), HttpError> {
        let mut state = self.state.lock().map_err(|_| {
            StreamableHttpError::Client(io::Error::other("MCP execution state unavailable"))
        })?;
        if bytes > self.max_bytes.saturating_sub(state.bytes) {
            drop(state);
            return Err(self.fail(DatadogError::QueryError(
                "MCP response byte budget exhausted".into(),
            )));
        }
        state.bytes += bytes;
        Ok(())
    }

    async fn send(
        &self,
        method: Method,
        uri: Arc<str>,
        session: Option<Arc<str>>,
        headers: HashMap<HeaderName, HeaderValue>,
        message: Option<&ClientJsonRpcMessage>,
        last_event: Option<String>,
    ) -> std::result::Result<reqwest::Response, HttpError> {
        if uri != self.uri {
            return Err(self.fail(DatadogError::InvalidInput(
                "MCP cannot change the authenticated origin or credentials".into(),
            )));
        }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(self.fail(DatadogError::TimeoutError));
        }
        {
            let mut state = self.state.lock().map_err(|_| {
                StreamableHttpError::Client(io::Error::other("MCP execution state unavailable"))
            })?;
            if state.calls >= self.max_calls {
                drop(state);
                return Err(self.fail(DatadogError::QueryError(
                    "MCP HTTP call budget exhausted".into(),
                )));
            }
            state.calls += 1;
        }
        let mut request = self
            .client
            .request(method, uri.as_ref())
            .header("accept", "application/json, text/event-stream");
        if let Some(session) = session {
            request = request.header("mcp-session-id", session.as_ref());
        }
        if let Some(last_event) = last_event {
            request = request.header("last-event-id", last_event);
        }
        for (name, value) in headers {
            if [
                "authorization",
                "dd-api-key",
                "dd-application-key",
                "host",
                "accept",
                "content-type",
                "content-length",
                "mcp-session-id",
                "last-event-id",
            ]
            .contains(&name.as_str())
            {
                return Err(self.fail(DatadogError::InvalidInput(
                    "Reserved MCP header override".into(),
                )));
            }
            request = request.header(name, value);
        }
        if let Some(message) = message {
            request = request.json(message);
        }
        tokio::time::timeout(remaining, request.send())
            .await
            .map_err(|_| self.fail(DatadogError::TimeoutError))?
            .map_err(|error| {
                self.fail(if error.is_timeout() {
                    DatadogError::TimeoutError
                } else {
                    DatadogError::NetworkError(error.without_url())
                })
            })
    }

    async fn bytes(
        &self,
        mut response: reqwest::Response,
    ) -> std::result::Result<Vec<u8>, HttpError> {
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| {
            self.fail(if error.is_timeout() {
                DatadogError::TimeoutError
            } else {
                DatadogError::NetworkError(error.without_url())
            })
        })? {
            self.charge_bytes(chunk.len())?;
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    async fn check_status(
        &self,
        response: reqwest::Response,
    ) -> std::result::Result<reqwest::Response, HttpError> {
        let status = response.status().as_u16();
        if (200..300).contains(&status) {
            return Ok(response);
        }
        let reset = response
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        let bytes = self.bytes(response).await?;
        let message = String::from_utf8_lossy(&bytes).into_owned();
        Err(self.fail(match status {
            401 => DatadogError::AuthError(message),
            403 => DatadogError::PermissionDenied(message),
            429 => DatadogError::RateLimitError { reset_secs: reset },
            _ => DatadogError::ApiError { status, message },
        }))
    }

    fn stream(
        &self,
        response: reqwest::Response,
        max_event: usize,
    ) -> BoxStream<'static, std::result::Result<Sse, sse_stream::Error>> {
        let state = self.clone();
        let ceiling = self.max_bytes.min(max_event);
        let mut received = 0_usize;
        let stream = response.bytes_stream().map(move |result| {
            let chunk = result.map_err(|error| {
                state.fail(if error.is_timeout() {
                    DatadogError::TimeoutError
                } else {
                    DatadogError::NetworkError(error.without_url())
                });
                io::Error::other("MCP response stream failed")
            })?;
            received = received.saturating_add(chunk.len());
            if received > ceiling {
                state.fail(DatadogError::QueryError(
                    "MCP stream byte budget exhausted".into(),
                ));
                return Err(io::Error::other("MCP stream byte budget exhausted"));
            }
            state
                .charge_bytes(chunk.len())
                .map_err(|_| io::Error::other("MCP response byte budget exhausted"))?;
            Ok(chunk)
        });
        SseStream::from_bytes_stream(stream).boxed()
    }
}

impl StreamableHttpClient for BoundedHttp {
    type Error = io::Error;

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> std::result::Result<StreamableHttpPostResponse, HttpError> {
        self.post_message_with_max_sse_event_size(
            uri,
            message,
            session_id,
            auth_header,
            custom_headers,
            self.max_bytes,
        )
        .await
    }

    async fn post_message_with_max_sse_event_size(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
        max_event: usize,
    ) -> std::result::Result<StreamableHttpPostResponse, HttpError> {
        if auth_header.is_some() {
            return Err(self.fail(DatadogError::InvalidInput(
                "MCP cannot override the selected credentials".into(),
            )));
        }
        let response = self
            .send(
                Method::POST,
                uri,
                session_id,
                custom_headers,
                Some(&message),
                None,
            )
            .await?;
        let response = self.check_status(response).await?;
        let session = response
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        if matches!(response.status().as_u16(), 202 | 204) {
            self.bytes(response).await?;
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        let media = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if media == "text/event-stream" {
            return Ok(StreamableHttpPostResponse::Sse(
                self.stream(response, max_event),
                session,
            ));
        }
        let bytes = self.bytes(response).await?;
        if bytes.is_empty() && !matches!(message, ClientJsonRpcMessage::Request(_)) {
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        if media != "application/json" {
            return Err(self.fail(DatadogError::DecodeError(
                "Unexpected MCP response content type".into(),
            )));
        }
        let message = serde_json::from_slice(&bytes).map_err(|_| {
            self.fail(DatadogError::DecodeError(
                "Invalid MCP JSON-RPC response".into(),
            ))
        })?;
        Ok(StreamableHttpPostResponse::Json(message, session))
    }

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Option<Arc<str>>,
        last_event_id: Option<String>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> std::result::Result<
        BoxStream<'static, std::result::Result<Sse, sse_stream::Error>>,
        HttpError,
    > {
        self.get_stream_with_max_sse_event_size(
            uri,
            session_id,
            last_event_id,
            auth_header,
            custom_headers,
            self.max_bytes,
        )
        .await
    }

    async fn get_stream_with_max_sse_event_size(
        &self,
        uri: Arc<str>,
        session_id: Option<Arc<str>>,
        last_event_id: Option<String>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
        max_event: usize,
    ) -> std::result::Result<
        BoxStream<'static, std::result::Result<Sse, sse_stream::Error>>,
        HttpError,
    > {
        if auth_header.is_some() {
            return Err(self.fail(DatadogError::InvalidInput(
                "MCP cannot override the selected credentials".into(),
            )));
        }
        let response = self
            .send(
                Method::GET,
                uri,
                session_id,
                custom_headers,
                None,
                last_event_id,
            )
            .await?;
        if response.status().as_u16() == 405 {
            self.bytes(response).await?;
            return Err(StreamableHttpError::ServerDoesNotSupportSse);
        }
        let response = self.check_status(response).await?;
        if !response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.split(';').next() == Some("text/event-stream"))
        {
            return Err(self.fail(DatadogError::DecodeError(
                "Unexpected MCP SSE response content type".into(),
            )));
        }
        Ok(self.stream(response, max_event))
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> std::result::Result<(), HttpError> {
        if auth_header.is_some() {
            return Err(self.fail(DatadogError::InvalidInput(
                "MCP cannot override the selected credentials".into(),
            )));
        }
        let response = self
            .send(
                Method::DELETE,
                uri,
                Some(session_id),
                custom_headers,
                None,
                None,
            )
            .await?;
        if !matches!(response.status().as_u16(), 404 | 405) {
            self.bytes(self.check_status(response).await?).await?;
        } else {
            self.bytes(response).await?;
        }
        Ok(())
    }
}
