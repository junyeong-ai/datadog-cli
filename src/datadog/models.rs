use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub status: String,
    pub res_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resp_version: Option<i32>,
    pub from_date: i64,
    pub to_date: i64,
    pub series: Vec<MetricSeries>,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<Vec<Option<f64>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub times: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_by: Option<Vec<String>>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct LogsResponse {
    pub data: Option<Vec<LogEntry>>,
    pub meta: Option<LogsMeta>,
    pub errors: Option<Vec<String>>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct LogsMeta {
    pub page: Option<LogsPage>,
    pub elapsed: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogsPage {
    pub after: Option<String>,
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
    pub events: Option<Vec<Event>>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<i64>,
    pub id_str: Option<String>,
    pub title: Option<String>,
    pub text: Option<String>,
    pub date_happened: Option<i64>,
    pub priority: Option<String>,
    pub host: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source: Option<String>,
    pub alert_type: Option<String>,
    pub comments: Option<Vec<String>>,
    pub device_name: Option<String>,
    pub is_aggregate: Option<bool>,
    pub monitor_group_status: Option<i32>,
    pub monitor_groups: Option<Vec<String>>,
    pub monitor_id: Option<i64>,
    pub resource: Option<String>,
    pub url: Option<String>,
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
pub struct ServicesResponse {
    pub data: Vec<Service>,
    pub meta: Option<ServicesMeta>,
    pub links: Option<ServicesLinks>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub service_type: Option<String>,
    pub attributes: Option<ServiceAttributes>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAttributes {
    pub schema_version: Option<String>,
    pub dd_service: Option<String>,
    pub dd_team: Option<String>,
    pub application: Option<String>,
    pub tier: Option<String>,
    pub lifecycle: Option<String>,
    pub type_of_service: Option<String>,
    pub languages: Option<Vec<String>>,
    pub contacts: Option<Vec<ServiceContact>>,
    pub links: Option<Vec<ServiceLink>>,
    pub repos: Option<Vec<ServiceRepo>>,
    pub docs: Option<Vec<ServiceDoc>>,
    pub tags: Option<Vec<String>>,
    pub integrations: Option<ServiceIntegrations>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceContact {
    pub name: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "type")]
    pub contact_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceLink {
    pub name: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "type")]
    pub link_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRepo {
    pub name: Option<String>,
    pub url: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDoc {
    pub name: Option<String>,
    pub url: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceIntegrations {
    pub pagerduty: Option<serde_json::Value>,
    pub slack: Option<serde_json::Value>,
    #[serde(flatten)]
    pub others: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesMeta {
    pub warnings: Option<Vec<ServicesWarning>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesWarning {
    pub code: Option<String>,
    pub detail: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesLinks {
    pub next: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsGroupBy {
    pub facet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<LogsGroupBySort>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub group_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsGroupBySort {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub sort_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumEventsResponse {
    pub data: Option<Vec<RumEvent>>,
    pub meta: Option<RumMeta>,
    pub links: Option<RumLinks>,
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
    pub application: Option<RumApplication>,
    pub view: Option<RumView>,
    pub session: Option<RumSession>,
    pub action: Option<RumAction>,
    pub resource: Option<RumResource>,
    pub error: Option<RumError>,
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumApplication {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumView {
    pub id: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
    pub url_path: Option<String>,
    pub time_spent: Option<i64>,
    pub loading_time: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumSession {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub session_type: Option<String>,
    pub has_replay: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumAction {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub action_type: Option<String>,
    pub target: Option<RumActionTarget>,
    pub loading_time: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumActionTarget {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumResource {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub resource_type: Option<String>,
    pub url: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<i32>,
    pub duration: Option<i64>,
    pub size: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumError {
    pub id: Option<String>,
    pub message: Option<String>,
    pub source: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub stack: Option<String>,
    pub is_crash: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumMeta {
    pub page: Option<RumPage>,
    pub elapsed: Option<i64>,
    pub request_id: Option<String>,
    pub status: Option<String>,
    pub warnings: Option<Vec<RumWarning>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumPage {
    pub after: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumLinks {
    pub next: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RumWarning {
    pub code: Option<String>,
    pub detail: Option<String>,
    pub title: Option<String>,
}
