use reqwest::{Client, Response, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::time::Duration;

use super::models::*;
use super::retry;
use crate::error::{DatadogError, Result};

const DEFAULT_TIMEOUT_SECS: u64 = 30;

pub struct DatadogClient {
    client: Client,
    api_key: String,
    app_key: String,
    base_url: String,
    tag_filter: Option<String>,
}

impl DatadogClient {
    pub fn new(api_key: String, app_key: String, site: Option<String>) -> Result<Self> {
        Self::with_tag_filter(api_key, app_key, site, std::env::var("DD_TAG_FILTER").ok())
    }

    pub fn with_tag_filter(
        api_key: String,
        app_key: String,
        site: Option<String>,
        tag_filter: Option<String>,
    ) -> Result<Self> {
        let site = site.unwrap_or_else(|| "datadoghq.com".to_string());
        let base_url = format!("https://api.{}", site);

        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(DatadogError::NetworkError)?;

        Ok(Self {
            client,
            api_key,
            app_key,
            base_url,
            tag_filter,
        })
    }

    pub fn get_tag_filter(&self) -> Option<&str> {
        self.tag_filter.as_deref()
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        query: Option<Vec<(&str, String)>>,
        body: Option<impl Serialize>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);

        let mut retries = 0;
        loop {
            let mut request = self
                .client
                .request(method.clone(), &url)
                .header("DD-API-KEY", &self.api_key)
                .header("DD-APPLICATION-KEY", &self.app_key)
                .header("Content-Type", "application/json");

            if let Some(ref params) = query {
                for (key, value) in params {
                    request = request.query(&[(key, value)]);
                }
            }

            if let Some(ref data) = body {
                request = request.json(data);
            }

            let response = request.send().await?;

            match self.handle_response(response).await {
                Ok(data) => return Ok(data),
                Err(e) => {
                    if !retry::should_retry(retries) {
                        return Err(e);
                    }

                    retries += 1;

                    // Exponential backoff
                    tokio::time::sleep(retry::calculate_backoff(retries)).await;
                }
            }
        }
    }

    async fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            response
                .json::<T>()
                .await
                .map_err(DatadogError::NetworkError)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            match status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    Err(DatadogError::AuthError(error_text))
                }
                StatusCode::TOO_MANY_REQUESTS => Err(DatadogError::RateLimitError),
                StatusCode::REQUEST_TIMEOUT => Err(DatadogError::TimeoutError),
                _ => Err(DatadogError::ApiError(format!(
                    "HTTP {}: {}",
                    status, error_text
                ))),
            }
        }
    }

    // ============= Metrics API =============

    pub async fn query_metrics(&self, query: &str, from: i64, to: i64) -> Result<MetricsResponse> {
        let params = vec![
            ("query", query.to_string()),
            ("from", from.to_string()),
            ("to", to.to_string()),
        ];

        self.request(
            reqwest::Method::GET,
            "/api/v1/query",
            Some(params),
            None::<()>,
        )
        .await
    }

    // ============= Logs API =============

    pub async fn search_logs(
        &self,
        query: &str,
        from: &str,
        to: &str,
        limit: Option<i32>,
    ) -> Result<LogsResponse> {
        let body = serde_json::json!({
            "filter": {
                "query": query,
                "from": from,
                "to": to
            },
            "page": {
                "limit": limit.unwrap_or(10)
            },
            "sort": "timestamp"
        });

        self.request(
            reqwest::Method::POST,
            "/api/v2/logs/events/search",
            None,
            Some(body),
        )
        .await
    }

    // ============= Monitors API =============

    pub async fn list_monitors(
        &self,
        tags: Option<String>,
        monitor_tags: Option<String>,
        page: Option<i32>,
        page_size: Option<i32>,
    ) -> Result<Vec<Monitor>> {
        let mut params = vec![];

        if let Some(t) = tags {
            params.push(("tags", t));
        }
        if let Some(mt) = monitor_tags {
            params.push(("monitor_tags", mt));
        }
        if let Some(p) = page {
            params.push(("page", p.to_string()));
        }
        if let Some(ps) = page_size {
            params.push(("page_size", ps.to_string()));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v1/monitor",
            if params.is_empty() {
                None
            } else {
                Some(params)
            },
            None::<()>,
        )
        .await
    }

    pub async fn get_monitor(&self, monitor_id: i64) -> Result<Monitor> {
        let endpoint = format!("/api/v1/monitor/{}", monitor_id);

        self.request(reqwest::Method::GET, &endpoint, None, None::<()>)
            .await
    }

    // ============= Events API =============

    pub async fn query_events(
        &self,
        start: i64,
        end: i64,
        priority: Option<String>,
        sources: Option<String>,
        tags: Option<String>,
    ) -> Result<EventsResponse> {
        let mut params = vec![("start", start.to_string()), ("end", end.to_string())];

        if let Some(p) = priority {
            params.push(("priority", p));
        }
        if let Some(s) = sources {
            params.push(("sources", s));
        }
        if let Some(t) = tags {
            params.push(("tags", t));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v1/events",
            Some(params),
            None::<()>,
        )
        .await
    }

    // ============= Infrastructure/Hosts API =============

    pub async fn list_hosts(
        &self,
        filter: Option<String>,
        from: Option<i64>,
        sort_field: Option<String>,
        sort_dir: Option<String>,
        start: Option<i32>,
        count: Option<i32>,
    ) -> Result<HostsResponse> {
        let mut params = vec![];

        if let Some(f) = filter {
            params.push(("filter", f));
        }
        if let Some(f) = from {
            params.push(("from", f.to_string()));
        }
        if let Some(sf) = sort_field {
            params.push(("sort_field", sf));
        }
        if let Some(sd) = sort_dir {
            params.push(("sort_dir", sd));
        }
        if let Some(s) = start {
            params.push(("start", s.to_string()));
        }
        if let Some(c) = count {
            params.push(("count", c.to_string()));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v1/hosts",
            if params.is_empty() {
                None
            } else {
                Some(params)
            },
            None::<()>,
        )
        .await
    }

    // ============= Dashboard API Methods =============

    /// List all dashboards
    pub async fn list_dashboards(&self) -> Result<DashboardsResponse> {
        self.request(
            reqwest::Method::GET,
            "/api/v1/dashboard",
            None::<Vec<(&str, String)>>,
            None::<()>,
        )
        .await
    }

    /// Get a specific dashboard by ID
    pub async fn get_dashboard(&self, dashboard_id: &str) -> Result<Dashboard> {
        let url = format!("/api/v1/dashboard/{}", dashboard_id);
        self.request(
            reqwest::Method::GET,
            &url,
            None::<Vec<(&str, String)>>,
            None::<()>,
        )
        .await
    }

    // ============= APM Spans API Methods =============

    /// List spans using the GET endpoint
    pub async fn list_spans(
        &self,
        query: &str,
        from: &str,
        to: &str,
        limit: Option<i32>,
        cursor: Option<String>,
        sort: Option<String>,
    ) -> Result<serde_json::Value> {
        let mut params = vec![
            ("filter[query]", query.to_string()),
            ("filter[from]", from.to_string()),
            ("filter[to]", to.to_string()),
            ("page[limit]", limit.unwrap_or(10).to_string()),
        ];

        // Add optional parameters
        if let Some(cursor_val) = cursor {
            params.push(("page[cursor]", cursor_val));
        }
        if let Some(sort_val) = sort {
            params.push(("sort", sort_val));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v2/spans/events",
            Some(params),
            None::<()>,
        )
        .await
    }

    // ============= Service Catalog API Methods =============

    /// Get service catalog with proper pagination
    pub async fn get_service_catalog(
        &self,
        page_size: Option<i32>,
        page_number: Option<i32>,
        filter_env: Option<String>,
    ) -> Result<ServicesResponse> {
        let mut params = vec![];

        // Use Datadog's pagination format for v2 API
        if let Some(size) = page_size {
            params.push(("page[size]", size.to_string()));
        }

        if let Some(number) = page_number {
            params.push(("page[number]", number.to_string()));
        }

        if let Some(env) = filter_env {
            params.push(("filter[env]", env));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v2/services/definitions",
            if params.is_empty() {
                None
            } else {
                Some(params)
            },
            None::<()>,
        )
        .await
    }

    // ============= Logs Analytics API Methods =============

    /// Aggregate log events into buckets and compute metrics
    pub async fn aggregate_logs(
        &self,
        query: &str,
        from: &str,
        to: &str,
        compute: Option<Vec<LogsCompute>>,
        group_by: Option<Vec<LogsGroupBy>>,
        timezone: Option<String>,
    ) -> Result<serde_json::Value> {
        let mut body = serde_json::json!({
            "filter": {
                "query": query,
                "from": from,
                "to": to
            }
        });

        if let Some(comp) = compute {
            body["compute"] = serde_json::to_value(comp)?;
        }

        if let Some(gb) = group_by {
            body["group_by"] = serde_json::to_value(gb)?;
        }

        if let Some(tz) = timezone {
            body["options"] = serde_json::json!({"timezone": tz});
        }

        // Debug: log request body
        log::debug!(
            "Logs aggregate request body: {}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );

        self.request(
            reqwest::Method::POST,
            "/api/v2/logs/analytics/aggregate",
            None,
            Some(body),
        )
        .await
    }

    // ============= RUM API Methods =============

    /// Search RUM events
    pub async fn search_rum_events(
        &self,
        query: &str,
        from: &str,
        to: &str,
        limit: Option<i32>,
        cursor: Option<String>,
        sort: Option<String>,
    ) -> Result<RumEventsResponse> {
        let mut body = serde_json::json!({
            "filter": {
                "query": query,
                "from": from,
                "to": to
            },
            "page": {
                "limit": limit.unwrap_or(10)
            }
        });

        if let Some(s) = sort {
            body["sort"] = serde_json::json!(s);
        }

        if let Some(c) = cursor {
            body["page"]["cursor"] = serde_json::json!(c);
        }

        self.request(
            reqwest::Method::POST,
            "/api/v2/rum/events/search",
            None,
            Some(body),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_new_with_default_site() {
        let client =
            DatadogClient::new("test_api_key".to_string(), "test_app_key".to_string(), None);

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url, "https://api.datadoghq.com");
        assert_eq!(client.api_key, "test_api_key");
        assert_eq!(client.app_key, "test_app_key");
    }

    #[tokio::test]
    async fn test_client_new_with_custom_site() {
        let client = DatadogClient::new(
            "test_api_key".to_string(),
            "test_app_key".to_string(),
            Some("datadoghq.eu".to_string()),
        );

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url, "https://api.datadoghq.eu");
    }

    #[test]
    fn test_client_regional_urls() {
        let regions = vec![
            ("datadoghq.com", "https://api.datadoghq.com"),
            ("datadoghq.eu", "https://api.datadoghq.eu"),
            ("us3.datadoghq.com", "https://api.us3.datadoghq.com"),
            ("us5.datadoghq.com", "https://api.us5.datadoghq.com"),
        ];

        for (region, expected_url) in regions {
            let client = DatadogClient::new(
                "key".to_string(),
                "app".to_string(),
                Some(region.to_string()),
            )
            .unwrap();

            assert_eq!(client.base_url, expected_url);
        }
    }

    #[test]
    fn test_tag_filter_injection() {
        let client = DatadogClient::with_tag_filter(
            "key".to_string(),
            "app".to_string(),
            None,
            Some("env:,service:".to_string()),
        )
        .unwrap();

        assert_eq!(client.get_tag_filter(), Some("env:,service:"));
    }

    #[test]
    fn test_no_tag_filter() {
        let client =
            DatadogClient::with_tag_filter("key".to_string(), "app".to_string(), None, None)
                .unwrap();

        assert_eq!(client.get_tag_filter(), None);
    }

    #[tokio::test]
    async fn test_handle_response_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok",
                "data": "test_value"
            })))
            .mount(&mock_server)
            .await;

        let mut client = DatadogClient::new("key".to_string(), "app".to_string(), None).unwrap();
        client.base_url = mock_server.uri();

        #[derive(serde::Deserialize)]
        struct TestResponse {
            status: String,
            data: String,
        }

        let result: Result<TestResponse> = client
            .request(reqwest::Method::GET, "/api/v1/test", None, None::<()>)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, "ok");
        assert_eq!(response.data, "test_value");
    }

    #[tokio::test]
    async fn test_handle_response_unauthorized() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&mock_server)
            .await;

        let mut client = DatadogClient::new("key".to_string(), "app".to_string(), None).unwrap();
        client.base_url = mock_server.uri();

        let result: Result<serde_json::Value> = client
            .request(reqwest::Method::GET, "/api/v1/test", None, None::<()>)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            DatadogError::AuthError(msg) => {
                assert!(msg.contains("Unauthorized"));
            }
            _ => panic!("Expected AuthError"),
        }
    }

    #[tokio::test]
    async fn test_handle_response_forbidden() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&mock_server)
            .await;

        let mut client = DatadogClient::new("key".to_string(), "app".to_string(), None).unwrap();
        client.base_url = mock_server.uri();

        let result: Result<serde_json::Value> = client
            .request(reqwest::Method::GET, "/api/v1/test", None, None::<()>)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            DatadogError::AuthError(msg) => {
                assert!(msg.contains("Forbidden"));
            }
            _ => panic!("Expected AuthError"),
        }
    }

    #[tokio::test]
    async fn test_handle_response_rate_limit() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
            .mount(&mock_server)
            .await;

        let mut client = DatadogClient::new("key".to_string(), "app".to_string(), None).unwrap();
        client.base_url = mock_server.uri();

        let result: Result<serde_json::Value> = client
            .request(reqwest::Method::GET, "/api/v1/test", None, None::<()>)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            DatadogError::RateLimitError => {}
            _ => panic!("Expected RateLimitError"),
        }
    }

    #[tokio::test]
    async fn test_handle_response_timeout() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(ResponseTemplate::new(408).set_body_string("Request timeout"))
            .mount(&mock_server)
            .await;

        let mut client = DatadogClient::new("key".to_string(), "app".to_string(), None).unwrap();
        client.base_url = mock_server.uri();

        let result: Result<serde_json::Value> = client
            .request(reqwest::Method::GET, "/api/v1/test", None, None::<()>)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            DatadogError::TimeoutError => {}
            _ => panic!("Expected TimeoutError"),
        }
    }

    #[tokio::test]
    async fn test_handle_response_server_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal server error"))
            .mount(&mock_server)
            .await;

        let mut client = DatadogClient::new("key".to_string(), "app".to_string(), None).unwrap();
        client.base_url = mock_server.uri();

        let result: Result<serde_json::Value> = client
            .request(reqwest::Method::GET, "/api/v1/test", None, None::<()>)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            DatadogError::ApiError(msg) => {
                assert!(msg.contains("HTTP 500"));
                assert!(msg.contains("Internal server error"));
            }
            _ => panic!("Expected ApiError"),
        }
    }

    #[tokio::test]
    async fn test_request_retry_logic() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(move |_req: &wiremock::Request| {
                let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    ResponseTemplate::new(500)
                } else {
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"}))
                }
            })
            .mount(&mock_server)
            .await;

        let mut client = DatadogClient::new("key".to_string(), "app".to_string(), None).unwrap();
        client.base_url = mock_server.uri();

        let result: Result<serde_json::Value> = client
            .request(reqwest::Method::GET, "/api/v1/test", None, None::<()>)
            .await;

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_request_max_retries() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(move |_req: &wiremock::Request| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(500)
            })
            .mount(&mock_server)
            .await;

        let mut client = DatadogClient::new("key".to_string(), "app".to_string(), None).unwrap();
        client.base_url = mock_server.uri();

        let result: Result<serde_json::Value> = client
            .request(reqwest::Method::GET, "/api/v1/test", None, None::<()>)
            .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn test_request_success_first_try() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        Mock::given(method("GET"))
            .and(path("/api/v1/test"))
            .respond_with(move |_req: &wiremock::Request| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"}))
            })
            .mount(&mock_server)
            .await;

        let mut client = DatadogClient::new("key".to_string(), "app".to_string(), None).unwrap();
        client.base_url = mock_server.uri();

        let result: Result<serde_json::Value> = client
            .request(reqwest::Method::GET, "/api/v1/test", None, None::<()>)
            .await;

        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
