use serde_json::{Value, json};

use crate::cli::ServicesArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter};

pub struct ServicesHandler;

impl ResponseFormatter for ServicesHandler {}

impl ServicesHandler {
    pub async fn list(client: &DatadogClient, args: &ServicesArgs) -> Result<Value> {
        let handler = ServicesHandler;

        let response = client
            .list_catalog_entities(
                &args.kind,
                args.name.as_deref(),
                args.owner.as_deref(),
                args.include.as_deref(),
                args.start,
                args.count,
            )
            .await?;

        // Catalog entities are passed through unmodified: their shape is
        // schema-version dependent (v3 entity model), so projecting fields
        // here would silently drop data.
        let data = response.get("data").cloned().unwrap_or_else(|| json!([]));
        let returned = data.as_array().map(|a| a.len()).unwrap_or(0);

        let pagination = PaginationInfo::from_offset_without_total(
            returned,
            args.start as usize,
            args.count as usize,
        );

        let mut meta = serde_json::Map::new();
        meta.insert("kind".to_string(), json!(args.kind));
        if let Some(included) = response.get("included").filter(|v| !v.is_null()) {
            meta.insert("included".to_string(), included.clone());
        }

        Ok(handler.format_list(
            data,
            Some(serde_json::to_value(pagination)?),
            Some(Value::Object(meta)),
        ))
    }
}
