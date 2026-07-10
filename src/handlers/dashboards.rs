use serde_json::{Value, json};

use crate::cli::DashboardsListArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter};

pub struct DashboardsHandler;

impl ResponseFormatter for DashboardsHandler {}

impl DashboardsHandler {
    pub async fn list(client: &DatadogClient, args: &DashboardsListArgs) -> Result<Value> {
        let handler = DashboardsHandler;

        let response = client
            .list_dashboards(
                Some(args.count),
                Some(args.start),
                args.filter_shared,
                args.filter_deleted,
            )
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

        let pagination = PaginationInfo::from_offset_without_total(
            data.len(),
            args.start as usize,
            args.count as usize,
        );

        Ok(handler.format_list(json!(data), Some(serde_json::to_value(pagination)?), None))
    }

    pub async fn get(client: &DatadogClient, dashboard_id: &str) -> Result<Value> {
        let handler = DashboardsHandler;

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
