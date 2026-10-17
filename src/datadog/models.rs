use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub status: String,
    pub series: Vec<MetricSeries>,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_by: Option<Vec<String>>,
}

impl MetricsResponse {
    pub(super) fn from_api_response(value: serde_json::Value) -> crate::error::Result<Self> {
        #[derive(Deserialize)]
        struct QueryStatus<'a> {
            status: Option<&'a str>,
            error: Option<&'a str>,
        }

        let status = QueryStatus::deserialize(&value)
            .map_err(|error| crate::error::DatadogError::DecodeError(error.to_string()))?;
        if let Some(error) = status.error.filter(|error| !error.is_empty()) {
            return Err(crate::error::DatadogError::QueryError(error.into()));
        }
        if let Some(status) = status.status.filter(|status| *status != "ok") {
            return Err(crate::error::DatadogError::QueryError(format!(
                "Query status: {status}"
            )));
        }
        serde_json::from_value(value)
            .map_err(|error| crate::error::DatadogError::DecodeError(error.to_string()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricSeries {
    pub metric: String,
    pub display_name: Option<String>,
    pub unit: Option<Vec<Option<Unit>>>,
    pub pointlist: Option<Vec<Vec<Option<f64>>>>,
    pub scope: String,
    pub expression: String,
    pub tag_set: Option<Vec<String>>,
    pub aggr: Option<String>,
    pub interval: Option<i64>,
    pub length: Option<i64>,
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub attributes: Option<HashMap<String, serde_json::Value>>,
    pub query_index: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Unit {
    pub family: String,
    pub name: String,
    pub plural: String,
    pub scale_factor: f64,
    pub short_name: Option<String>,
    pub id: Option<i64>,
}

/// Cursor pagination envelope shared by the v2 search APIs
/// (logs, events, RUM): the next-page cursor lives at `meta.page.after`.
#[derive(Debug, Serialize, Deserialize)]
pub struct CursorMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<CursorPage>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CursorPage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogsResponse {
    pub data: Option<Vec<LogEntry>>,
    pub meta: Option<CursorMeta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub log_type: Option<String>,
    pub attributes: Option<LogAttributes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogAttributes {
    pub timestamp: Option<String>,
    pub tags: Option<Vec<String>>,
    pub host: Option<String>,
    pub service: Option<String>,
    pub message: Option<String>,
    pub status: Option<String>,
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitor {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub monitor_type: String,
    pub query: String,
    pub message: Option<String>,
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub created_at: Option<i64>,
    pub modified: Option<String>,
    pub overall_state: Option<String>,
    pub overall_state_modified: Option<String>,
    pub priority: Option<i32>,
    pub options: Option<MonitorOptions>,
    pub creator: Option<Creator>,
    pub deleted: Option<String>,
    pub multi: Option<bool>,
    pub org_id: Option<i64>,
    pub restricted_roles: Option<Vec<String>>,
    pub matching_downtimes: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creator {
    pub id: Option<i64>,
    pub email: Option<String>,
    pub handle: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorOptions {
    pub thresholds: Option<MonitorThresholds>,
    pub notify_no_data: Option<bool>,
    pub notify_audit: Option<bool>,
    pub timeout_h: Option<i32>,
    pub silenced: Option<serde_json::Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorThresholds {
    pub critical: Option<f64>,
    pub warning: Option<f64>,
    pub ok: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventsResponse {
    pub data: Option<Vec<Event>>,
    pub meta: Option<CursorMeta>,
}

/// v2 event. Attributes vary by event category (only `timestamp`,
/// `message`, `tags`, and the nested `attributes` object are stable),
/// so they are kept as raw JSON.
#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub attributes: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostsResponse {
    pub total_matching: i64,
    pub total_returned: i64,
    pub host_list: Vec<Host>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: Option<i64>,
    pub name: String,
    pub up: bool,
    pub is_muted: bool,
    pub tags_by_source: Option<HashMap<String, Vec<String>>>,
    pub apps: Option<Vec<String>>,
    pub aws_name: Option<String>,
    pub host_name: String,
    pub last_reported_time: Option<i64>,
    pub sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardsResponse {
    pub dashboards: Vec<DashboardSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub id: String,
    pub title: String,
    pub url: String,
    pub author_handle: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub is_read_only: Option<bool>,
    pub layout_type: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub author_info: Option<AuthorInfo>,
    pub layout_type: String,
    pub url: String,
    pub is_read_only: Option<bool>,
    pub template_variables: Option<Vec<TemplateVariable>>,
    pub widgets: Vec<Widget>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    pub name: Option<String>,
    pub handle: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    #[serde(rename = "default")]
    pub default_value: Option<String>,
    pub prefix: Option<String>,
    pub available_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: Option<i64>,
    pub definition: WidgetDefinition,
    pub layout: Option<WidgetLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetDefinition {
    #[serde(rename = "type")]
    pub widget_type: String,
    pub title: Option<String>,
    pub title_size: Option<String>,
    pub title_align: Option<String>,
    pub requests: Option<Vec<serde_json::Value>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetLayout {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsCompute {
    pub aggregation: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub compute_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumEventsResponse {
    pub data: Option<Vec<RumEvent>>,
    pub meta: Option<CursorMeta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub attributes: Option<RumAttributes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumAttributes {
    pub timestamp: Option<String>,
    pub tags: Option<Vec<String>>,
    pub service: Option<String>,
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FormulaResponse {
    data: Option<serde_json::Value>,
    errors: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FormulaData<T> {
    attributes: T,
}

impl FormulaResponse {
    pub(super) fn into_attributes<T: serde::de::DeserializeOwned>(self) -> crate::error::Result<T> {
        if let Some(message) = self.errors.filter(|message| !message.is_empty()) {
            return Err(crate::error::DatadogError::QueryError(message));
        }
        let data = self.data.ok_or_else(|| {
            crate::error::DatadogError::DecodeError("Metric response is missing data".into())
        })?;
        serde_json::from_value::<FormulaData<T>>(data)
            .map(|data| data.attributes)
            .map_err(|error| crate::error::DatadogError::DecodeError(error.to_string()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeseriesAttributes {
    pub times: Vec<i64>,
    pub series: Vec<serde_json::Map<String, serde_json::Value>>,
    pub values: Vec<Vec<Option<serde_json::Number>>>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl TimeseriesAttributes {
    pub(super) fn validate(&self) -> crate::error::Result<()> {
        if self.series.len() != self.values.len()
            || self
                .values
                .iter()
                .any(|values| values.len() != self.times.len())
        {
            return Err(crate::error::DatadogError::DecodeError(
                "Timeseries response arrays have inconsistent lengths".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScalarAttributes {
    pub columns: Vec<ScalarColumn>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ScalarColumn {
    #[serde(rename = "group")]
    Group {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        values: Vec<Vec<String>>,
        #[serde(flatten)]
        extra: serde_json::Map<String, serde_json::Value>,
    },
    #[serde(rename = "number")]
    Number {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        values: Vec<Option<serde_json::Number>>,
        #[serde(flatten)]
        extra: serde_json::Map<String, serde_json::Value>,
    },
}

impl ScalarColumn {
    fn row_count(&self) -> usize {
        match self {
            Self::Group { values, .. } => values.len(),
            Self::Number { values, .. } => values.len(),
        }
    }
}

impl ScalarAttributes {
    pub(super) fn validate(&self) -> crate::error::Result<()> {
        if let Some(first) = self.columns.first()
            && self
                .columns
                .iter()
                .any(|column| column.row_count() != first.row_count())
        {
            return Err(crate::error::DatadogError::DecodeError(
                "Scalar response columns have inconsistent row counts".into(),
            ));
        }
        Ok(())
    }
}
