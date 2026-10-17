use super::models::*;
pub use super::transport::DatadogClient;
use crate::error::Result;

/// Common parameters for cursor-paginated search endpoints.
pub struct SearchParams<'a> {
    pub query: &'a str,
    pub from: &'a str,
    pub to: &'a str,
    pub limit: i32,
    pub cursor: Option<&'a str>,
    pub sort: Option<&'a str>,
}

impl DatadogClient {
    /// Builds the common request body for the v2 cursor-paginated search
    /// endpoints: `filter.{query,from,to}` + `page.{limit,cursor}` + `sort`.
    fn search_body(params: &SearchParams<'_>) -> serde_json::Value {
        let mut body = serde_json::json!({
            "filter": {
                "query": params.query,
                "from": params.from,
                "to": params.to
            },
            "page": {
                "limit": params.limit
            }
        });

        if let Some(c) = params.cursor {
            body["page"]["cursor"] = serde_json::json!(c);
        }

        if let Some(s) = params.sort {
            body["sort"] = serde_json::json!(s);
        }

        body
    }

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

    /// Formula reference names for v2 queries: a, b, c, ...
    fn metric_query_name(index: usize) -> String {
        char::from(b'a' + index as u8).to_string()
    }

    /// v2 cross-product timeseries query with formula support.
    /// `from_ms`/`to_ms` are epoch milliseconds; `interval_ms` optional.
    pub async fn query_timeseries(
        &self,
        queries: &[String],
        formulas: &[String],
        from_ms: i64,
        to_ms: i64,
        interval_ms: Option<i64>,
    ) -> Result<serde_json::Value> {
        let mut attributes = serde_json::json!({
            "from": from_ms,
            "to": to_ms,
            "queries": queries.iter().enumerate().map(|(i, q)| {
                serde_json::json!({
                    "data_source": "metrics",
                    "query": q,
                    "name": Self::metric_query_name(i)
                })
            }).collect::<Vec<_>>()
        });

        if let Some(interval) = interval_ms {
            attributes["interval"] = serde_json::json!(interval);
        }

        if !formulas.is_empty() {
            attributes["formulas"] = serde_json::json!(
                formulas
                    .iter()
                    .map(|f| serde_json::json!({ "formula": f }))
                    .collect::<Vec<_>>()
            );
        }

        let body = serde_json::json!({
            "data": {
                "type": "timeseries_request",
                "attributes": attributes
            }
        });

        self.request(
            reqwest::Method::POST,
            "/api/v2/query/timeseries",
            None,
            Some(body),
        )
        .await
    }

    /// v2 scalar query: each metric query is reduced to a single value
    /// with `aggregator` over the window. `from_ms`/`to_ms` epoch millis.
    pub async fn query_scalar(
        &self,
        queries: &[String],
        formulas: &[String],
        aggregator: &str,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<serde_json::Value> {
        let mut attributes = serde_json::json!({
            "from": from_ms,
            "to": to_ms,
            "queries": queries.iter().enumerate().map(|(i, q)| {
                serde_json::json!({
                    "data_source": "metrics",
                    "query": q,
                    "aggregator": aggregator,
                    "name": Self::metric_query_name(i)
                })
            }).collect::<Vec<_>>()
        });

        if !formulas.is_empty() {
            attributes["formulas"] = serde_json::json!(
                formulas
                    .iter()
                    .map(|f| serde_json::json!({ "formula": f }))
                    .collect::<Vec<_>>()
            );
        }

        let body = serde_json::json!({
            "data": {
                "type": "scalar_request",
                "attributes": attributes
            }
        });

        self.request(
            reqwest::Method::POST,
            "/api/v2/query/scalar",
            None,
            Some(body),
        )
        .await
    }

    // ============= Logs API =============

    pub async fn search_logs(
        &self,
        params: &SearchParams<'_>,
        storage_tier: Option<&str>,
    ) -> Result<LogsResponse> {
        let mut body = Self::search_body(params);

        if let Some(tier) = storage_tier {
            body["filter"]["storage_tier"] = serde_json::json!(tier);
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
        compute: Vec<LogsCompute>,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "filter": {
                "query": query,
                "from": from,
                "to": to
            },
            "compute": compute
        });

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
        tags: Option<&str>,
        monitor_tags: Option<&str>,
        page: Option<i32>,
        page_size: Option<i32>,
    ) -> Result<Vec<Monitor>> {
        let mut params = vec![];

        if let Some(t) = tags {
            params.push(("tags", t.to_string()));
        }
        if let Some(mt) = monitor_tags {
            params.push(("monitor_tags", mt.to_string()));
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

    pub async fn search_events(&self, params: &SearchParams<'_>) -> Result<EventsResponse> {
        let body = Self::search_body(params);

        self.request(
            reqwest::Method::POST,
            "/api/v2/events/search",
            None,
            Some(body),
        )
        .await
    }

    // ============= Hosts API =============

    pub async fn list_hosts(
        &self,
        filter: Option<&str>,
        from: Option<i64>,
        sort_field: Option<&str>,
        sort_dir: Option<&str>,
        start: Option<i32>,
        count: Option<i32>,
    ) -> Result<HostsResponse> {
        let mut params = vec![];

        if let Some(f) = filter {
            params.push(("filter", f.to_string()));
        }
        if let Some(f) = from {
            params.push(("from", f.to_string()));
        }
        if let Some(sf) = sort_field {
            params.push(("sort_field", sf.to_string()));
        }
        if let Some(sd) = sort_dir {
            params.push(("sort_dir", sd.to_string()));
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
        filter_shared: bool,
        filter_deleted: bool,
    ) -> Result<DashboardsResponse> {
        let mut params = vec![];

        if let Some(c) = count {
            params.push(("count", c.to_string()));
        }
        if let Some(s) = start {
            params.push(("start", s.to_string()));
        }
        if filter_shared {
            params.push(("filter[shared]", "true".to_string()));
        }
        if filter_deleted {
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

    pub async fn search_spans(&self, params: &SearchParams<'_>) -> Result<serde_json::Value> {
        let attributes = Self::search_body(params);

        let body = serde_json::json!({
            "data": {
                "type": "search_request",
                "attributes": attributes
            }
        });

        self.request(
            reqwest::Method::POST,
            "/api/v2/spans/events/search",
            None,
            Some(body),
        )
        .await
    }

    // ============= Software Catalog API =============

    pub async fn list_catalog_entities(
        &self,
        kind: &str,
        name: Option<&str>,
        owner: Option<&str>,
        include: Option<&str>,
        offset: i32,
        limit: i32,
    ) -> Result<serde_json::Value> {
        let mut params = vec![
            ("filter[kind]", kind.to_string()),
            ("page[offset]", offset.to_string()),
            ("page[limit]", limit.to_string()),
        ];

        if let Some(n) = name {
            params.push(("filter[name]", n.to_string()));
        }
        if let Some(o) = owner {
            params.push(("filter[owner]", o.to_string()));
        }
        if let Some(i) = include {
            params.push(("include", i.to_string()));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v2/catalog/entity",
            Some(params),
            None::<()>,
        )
        .await
    }

    // ============= RUM API =============

    pub async fn search_rum_events(&self, params: &SearchParams<'_>) -> Result<RumEventsResponse> {
        let body = Self::search_body(params);

        self.request(
            reqwest::Method::POST,
            "/api/v2/rum/events/search",
            None,
            Some(body),
        )
        .await
    }

    // ============= SLO API =============

    pub async fn list_slos(
        &self,
        query: Option<&str>,
        tags_query: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<serde_json::Value> {
        let mut params = vec![("limit", limit.to_string()), ("offset", offset.to_string())];

        if let Some(q) = query {
            params.push(("query", q.to_string()));
        }
        if let Some(tq) = tags_query {
            params.push(("tags_query", tq.to_string()));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v1/slo",
            Some(params),
            None::<()>,
        )
        .await
    }

    pub async fn get_slo(&self, slo_id: &str) -> Result<serde_json::Value> {
        let endpoint = format!("/api/v1/slo/{}", slo_id);
        self.request(reqwest::Method::GET, &endpoint, None, None::<()>)
            .await
    }

    // ============= Incidents API =============

    pub async fn list_incidents(
        &self,
        size: i32,
        offset: i32,
        include: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut params = vec![
            ("page[size]", size.to_string()),
            ("page[offset]", offset.to_string()),
        ];

        if let Some(i) = include {
            params.push(("include", i.to_string()));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v2/incidents",
            Some(params),
            None::<()>,
        )
        .await
    }

    pub async fn get_incident(
        &self,
        incident_id: &str,
        include: Option<&str>,
    ) -> Result<serde_json::Value> {
        let endpoint = format!("/api/v2/incidents/{}", incident_id);
        let params = include.map(|i| vec![("include", i.to_string())]);

        self.request(reqwest::Method::GET, &endpoint, params, None::<()>)
            .await
    }

    // ============= Error Tracking API =============

    /// Single-shot search: this endpoint has no pagination.
    /// `from_ms`/`to_ms` are epoch milliseconds (from inclusive, to exclusive).
    pub async fn search_error_issues(
        &self,
        query: &str,
        track: &str,
        from_ms: i64,
        to_ms: i64,
        include: Option<&str>,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "data": {
                "type": "search_request",
                "attributes": {
                    "query": query,
                    "track": track,
                    "from": from_ms,
                    "to": to_ms
                }
            }
        });

        let params = include.map(|i| vec![("include", i.to_string())]);

        self.request(
            reqwest::Method::POST,
            "/api/v2/error-tracking/issues/search",
            params,
            Some(body),
        )
        .await
    }

    pub async fn get_error_issue(&self, issue_id: &str) -> Result<serde_json::Value> {
        let endpoint = format!("/api/v2/error-tracking/issues/{}", issue_id);
        self.request(reqwest::Method::GET, &endpoint, None, None::<()>)
            .await
    }

    // ============= Downtimes API =============

    pub async fn list_downtimes(
        &self,
        current_only: bool,
        offset: i32,
        limit: i32,
    ) -> Result<serde_json::Value> {
        let mut params = vec![
            ("page[offset]", offset.to_string()),
            ("page[limit]", limit.to_string()),
        ];

        if current_only {
            params.push(("current_only", "true".to_string()));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v2/downtime",
            Some(params),
            None::<()>,
        )
        .await
    }

    // ============= Audit API =============

    pub async fn search_audit_events(
        &self,
        params: &SearchParams<'_>,
    ) -> Result<serde_json::Value> {
        let body = Self::search_body(params);

        self.request(
            reqwest::Method::POST,
            "/api/v2/audit/events/search",
            None,
            Some(body),
        )
        .await
    }

    // ============= Teams API =============

    pub async fn list_teams(
        &self,
        keyword: Option<&str>,
        me: bool,
        page_number: i32,
        page_size: i32,
    ) -> Result<serde_json::Value> {
        let mut params = vec![
            ("page[number]", page_number.to_string()),
            ("page[size]", page_size.to_string()),
        ];

        if let Some(k) = keyword {
            params.push(("filter[keyword]", k.to_string()));
        }
        if me {
            params.push(("filter[me]", "true".to_string()));
        }

        self.request(
            reqwest::Method::GET,
            "/api/v2/team",
            Some(params),
            None::<()>,
        )
        .await
    }

    // ============= LLM Observability API =============

    /// Preview endpoint ("subject to change" per Datadog docs). Requires the
    /// JSON:API content type and a `type: "spans"` envelope — this differs
    /// from the APM spans endpoint's `search_request` type.
    pub async fn search_llm_obs_spans(
        &self,
        params: &SearchParams<'_>,
        ml_app: Option<&str>,
        span_kind: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut attributes = Self::search_body(params);

        if let Some(app) = ml_app {
            attributes["filter"]["ml_app"] = serde_json::json!(app);
        }
        if let Some(kind) = span_kind {
            attributes["filter"]["span_kind"] = serde_json::json!(kind);
        }

        let body = serde_json::json!({
            "data": {
                "type": "spans",
                "attributes": attributes
            }
        });

        self.request_with_content_type(
            reqwest::Method::POST,
            "/api/v2/llm-obs/v1/spans/events/search",
            None,
            Some(body),
            "application/vnd.api+json",
        )
        .await
    }
}
