use serde_json::{Value, json};

use crate::cli::DashboardsListArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::ResponseFormatter;
use crate::pagination::PaginationInfo;

pub struct DashboardsHandler;

impl ResponseFormatter for DashboardsHandler {}

impl DashboardsHandler {
    pub async fn list(client: &DatadogClient, args: &DashboardsListArgs) -> Result<Value> {
        let handler = DashboardsHandler;

        let response = client
            .list_dashboards(
                Some(args.limit),
                Some(args.offset),
                args.filter_shared,
                args.filter_deleted,
            )
            .await?;

        let data: Vec<Value> = response.dashboards.into_iter().map(Value::Object).collect();

        let pagination = PaginationInfo::from_offset(
            data.len(),
            args.offset as usize,
            args.limit as usize,
            None,
        );

        Ok(handler.format_list(json!(data), Some(serde_json::to_value(pagination)?), None))
    }

    pub async fn get(client: &DatadogClient, dashboard_id: &str) -> Result<Value> {
        let handler = DashboardsHandler;

        let dashboard = client.get_dashboard(dashboard_id).await?;
        Ok(handler.format_detail(serde_json::to_value(dashboard)?))
    }
}
