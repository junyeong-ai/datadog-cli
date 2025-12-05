use reqwest::{Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

use super::models::*;
use super::retry;
use crate::error::{DatadogError, Result};

pub struct DatadogClient {
    client: Client,
    api_key: String,
    app_key: String,
    base_url: String,
    max_retries: u32,
    tag_filter: Option<String>,
}

impl DatadogClient {
    pub fn new(
        api_key: String,
        app_key: String,
        site: Option<String>,
        timeout_secs: u64,
        max_retries: u32,
        tag_filter: Option<String>,
    ) -> Result<Self> {
        let site = site.unwrap_or_else(|| "datadoghq.com".to_string());
        let base_url = format!("https://api.{}", site);

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(DatadogError::NetworkError)?;

        Ok(Self {
            client,
            api_key,
            app_key,
            base_url,
            max_retries,
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
                    if !retry::should_retry(retries, self.max_retries) {
                        return Err(e);
                    }
                    retries += 1;
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
        limit: i32,
        cursor: Option<String>,
        sort: Option<String>,
    ) -> Result<LogsResponse> {
        let mut body = serde_json::json!({
            "filter": {
                "query": query,
                "from": from,
                "to": to
            },
            "page": {
                "limit": limit
            }
        });

        if let Some(c) = cursor {
            body["page"]["cursor"] = serde_json::json!(c);
        }

        if let Some(s) = sort {
            body["sort"] = serde_json::json!(s);
        }

        self.request(
            reqwest::Method::POST,
            "/api/v2/logs/events/search",
            None,
            Some(body),
        )
        .await
    }

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

        self.request(
            reqwest::Method::POST,
            "/api/v2/logs/analytics/aggregate",
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

    // ============= Hosts API =============

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

    // ============= Dashboard API =============

    pub async fn list_dashboards(
        &self,
        count: Option<i32>,
        start: Option<i32>,
        filter_shared: Option<bool>,
        filter_deleted: Option<bool>,
    ) -> Result<DashboardsResponse> {
        let mut params = vec![];

        if let Some(c) = count {
            params.push(("count", c.to_string()));
        }
        if let Some(s) = start {
            params.push(("start", s.to_string()));
        }
        if let Some(true) = filter_shared {
            params.push(("filter[shared]", "true".to_string()));
        }
        if let Some(true) = filter_deleted {
            params.push(("filter[deleted]", "true".to_string()));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v1/dashboard",
            if params.is_empty() {
                None
            } else {
                Some(params)
            },
            None::<()>,
        )
        .await
    }

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

    // ============= APM Spans API =============

    pub async fn list_spans(
        &self,
        query: &str,
        from: &str,
        to: &str,
        limit: i32,
        cursor: Option<String>,
        sort: Option<String>,
    ) -> Result<serde_json::Value> {
        let mut params = vec![
            ("filter[query]", query.to_string()),
            ("filter[from]", from.to_string()),
            ("filter[to]", to.to_string()),
            ("page[limit]", limit.to_string()),
        ];

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

    // ============= Service Catalog API =============

    pub async fn get_service_catalog(
        &self,
        page_size: Option<i32>,
        page_number: Option<i32>,
        filter_env: Option<String>,
    ) -> Result<ServicesResponse> {
        let mut params = vec![];

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

    // ============= RUM API =============

    pub async fn search_rum_events(
        &self,
        query: &str,
        from: &str,
        to: &str,
        limit: i32,
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
                "limit": limit
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
        let client = DatadogClient::new(
            "test_api_key".to_string(),
            "test_app_key".to_string(),
            None,
            30,
            3,
            None,
        );

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url, "https://api.datadoghq.com");
    }

    #[tokio::test]
    async fn test_client_new_with_custom_site() {
        let client = DatadogClient::new(
            "test_api_key".to_string(),
            "test_app_key".to_string(),
            Some("datadoghq.eu".to_string()),
            30,
            3,
            None,
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
                30,
                3,
                None,
            )
            .unwrap();

            assert_eq!(client.base_url, expected_url);
        }
    }

    #[test]
    fn test_tag_filter() {
        let client = DatadogClient::new(
            "key".to_string(),
            "app".to_string(),
            None,
            30,
            3,
            Some("env:,service:".to_string()),
        )
        .unwrap();

        assert_eq!(client.get_tag_filter(), Some("env:,service:"));
    }

    #[test]
    fn test_custom_timeout_and_retries() {
        let client = DatadogClient::new(
            "key".to_string(),
            "app".to_string(),
            None,
            60,
            5,
            None,
        )
        .unwrap();

        assert_eq!(client.max_retries, 5);
    }
}
