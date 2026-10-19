use serde_json::{Value, json};

use crate::cli::HostsArgs;
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter, TagFilter};

pub struct HostsHandler;

impl TagFilter for HostsHandler {}
impl ResponseFormatter for HostsHandler {}

impl HostsHandler {
    pub async fn list(client: &DatadogClient, args: &HostsArgs) -> Result<Value> {
        let handler = HostsHandler;

        let from = crate::utils::parse_time(&args.from)?;

        let response = client
            .list_hosts(
                args.filter.as_deref(),
                Some(from),
                args.sort_field.as_deref(),
                args.sort_dir.as_deref(),
                Some(args.start),
                Some(args.count),
            )
            .await?;

        let tag_filter = handler.resolve_tag_filter(args.tag_filter.as_deref(), client);

        let hosts = response
            .host_list
            .iter()
            .map(|host| -> Result<Value> {
                let filtered_tags =
                    handler.filter_tags_map(host.tags_by_source.as_ref(), tag_filter);
                let mut host_json = serde_json::to_value(host)?;
                if let Some(fields) = host_json.as_object_mut() {
                    fields.remove("tags_by_source");
                }
                host_json["last_reported"] =
                    json!(host.last_reported_time.map(crate::utils::format_timestamp));
                if let Some(tags) = filtered_tags
                    && !tags.is_empty()
                {
                    host_json["tags"] = json!(tags);
                }
                Ok(host_json)
            })
            .collect::<Result<Vec<_>>>()?;
        let data = json!(hosts);

        let total = usize::try_from(response.total_matching).map_err(|_| {
            crate::error::DatadogError::DecodeError(
                "Host total_matching must be nonnegative".into(),
            )
        })?;
        let pagination = PaginationInfo::from_offset_total(
            response.host_list.len(),
            args.start as usize,
            args.count as usize,
            Some(total),
        );

        Ok(json!({
            "data": data,
            "pagination": pagination
        }))
    }
}
