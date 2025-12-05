use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ParameterParser, ResponseFormatter};

pub struct DashboardsHandler;

impl ResponseFormatter for DashboardsHandler {}
impl ParameterParser for DashboardsHandler {}

impl DashboardsHandler {
    pub async fn list(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = DashboardsHandler;

        let count = handler.extract_i32(params, "count", 100);
        let start = handler.extract_i32(params, "start", 0);
        let filter_shared = params["filter_shared"].as_bool();
        let filter_deleted = params["filter_deleted"].as_bool();

        let response = client
            .list_dashboards(Some(count), Some(start), filter_shared, filter_deleted)
            .await?;

        let data: Vec<Value> = response
            .dashboards
            .iter()
            .map(|d| {
                json!({
                    "id": d.id,
                    "title": d.title,
                    "description": d.description,
                    "layout_type": d.layout_type,
                    "url": d.url,
                    "created": d.created_at,
                    "modified": d.modified_at,
                })
            })
            .collect();

        let pagination = PaginationInfo::from_offset(data.len(), start as usize, count as usize);

        Ok(handler.format_list(json!(data), Some(serde_json::to_value(pagination)?), None))
    }

    pub async fn get(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = DashboardsHandler;

        let dashboard_id = params["dashboard_id"].as_str().ok_or_else(|| {
            crate::error::DatadogError::InvalidInput("Missing dashboard_id".into())
        })?;

        let d = client.get_dashboard(dashboard_id).await?;

        let data = json!({
            "id": d.id,
            "title": d.title,
            "description": d.description,
            "layout_type": d.layout_type,
            "widgets": d.widgets,
            "template_variables": d.template_variables,
            "author": d.author_info,
            "created": d.created_at,
            "modified": d.modified_at,
            "url": d.url,
        });

        Ok(handler.format_detail(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_parser() {
        let handler = DashboardsHandler;
        let params = json!({
            "count": 50,
            "start": 10,
            "filter_shared": true,
        });

        assert_eq!(handler.extract_i32(&params, "count", 100), 50);
        assert_eq!(handler.extract_i32(&params, "start", 0), 10);
    }
}
