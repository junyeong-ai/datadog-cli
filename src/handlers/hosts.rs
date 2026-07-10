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

        let data = json!(response.host_list.iter().map(|host| {
            let filtered_tags = handler.filter_tags_map(host.tags_by_source.as_ref(), tag_filter);

            let mut host_json = json!({
                "name": host.name,
                "host_name": host.host_name,
                "up": host.up,
                "is_muted": host.is_muted,
                "last_reported": host.last_reported_time.map(crate::utils::format_timestamp),
                "aws_name": host.aws_name,
                "apps": host.apps,
                "sources": host.sources,
            });

            // Only add tags if not empty
            if let Some(tags) = filtered_tags
                && !tags.is_empty() {
                    host_json["tags"] = json!(tags);
                }

            host_json
        }).collect::<Vec<_>>());

        let pagination = PaginationInfo::from_offset(
            response.total_matching as usize,
            args.start as usize,
            args.count as usize,
        );

        Ok(json!({
            "data": data,
            "pagination": pagination
        }))
    }
}
