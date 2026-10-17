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
            .into_iter()
            .map(|mut dashboard| {
                add_timestamp_aliases(&mut dashboard);
                Value::Object(dashboard)
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

        let dashboard = client.get_dashboard(dashboard_id).await?;
        let mut data = serde_json::to_value(dashboard)?;
        if let Value::Object(fields) = &mut data {
            add_timestamp_aliases(fields);
            let author =
                if fields.contains_key("author_handle") || fields.contains_key("author_name") {
                    json!({
                        "name": fields.get("author_name"),
                        "handle": fields.get("author_handle"),
                    })
                } else {
                    Value::Null
                };
            fields.entry("author").or_insert(author);
        }

        Ok(handler.format_detail(data))
    }
}

fn add_timestamp_aliases(fields: &mut serde_json::Map<String, Value>) {
    for (alias, source) in [("created", "created_at"), ("modified", "modified_at")] {
        let value = fields.get(source).cloned().unwrap_or(Value::Null);
        fields.entry(alias).or_insert(value);
    }
}
