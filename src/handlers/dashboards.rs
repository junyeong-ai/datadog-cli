use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::ResponseFormatter;

pub struct DashboardsHandler;

impl ResponseFormatter for DashboardsHandler {}

impl DashboardsHandler {
    pub async fn list(client: Arc<DatadogClient>, _params: &Value) -> Result<Value> {
        let handler = DashboardsHandler;

        let response = client.list_dashboards().await?;
        let dashboards = response.dashboards;

        let data = json!(
            dashboards
                .iter()
                .map(|dashboard| {
                    json!({
                        "id": dashboard.id,
                        "title": dashboard.title,
                        "description": dashboard.description,
                        "layout_type": dashboard.layout_type,
                        "url": dashboard.url,
                        "created": dashboard.created_at,
                        "modified": dashboard.modified_at
                    })
                })
                .collect::<Vec<_>>()
        );

        Ok(handler.format_detail(data))
    }

    pub async fn get(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = DashboardsHandler;

        let dashboard_id = params["dashboard_id"].as_str().ok_or_else(|| {
            crate::error::DatadogError::InvalidInput("Missing 'dashboard_id' parameter".to_string())
        })?;

        let response = client.get_dashboard(dashboard_id).await?;

        let data = json!({
            "id": response.id,
            "title": response.title,
            "description": response.description,
            "layout_type": response.layout_type,
            "widgets": response.widgets,
            "template_variables": response.template_variables,
            "author": response.author_info,
            "created": response.created_at,
            "modified": response.modified_at,
            "url": response.url
        });

        Ok(handler.format_detail(data))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_get_missing_dashboard_id() {
        let params = json!({});
        let dashboard_id = params["dashboard_id"].as_str();
        assert_eq!(dashboard_id, None);
    }

    #[test]
    fn test_get_valid_dashboard_id() {
        let params = json!({"dashboard_id": "abc-123-def"});
        let dashboard_id = params["dashboard_id"].as_str();
        assert_eq!(dashboard_id, Some("abc-123-def"));
    }
}
