use serde_json::{Value, json};

use crate::cli::{MetricsArgs, ScalarArgs, TimeseriesArgs};
use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{ResponseFormatter, TimeHandler};

pub struct MetricsHandler;

impl TimeHandler for MetricsHandler {}
impl ResponseFormatter for MetricsHandler {}

impl MetricsHandler {
    pub async fn query(client: &DatadogClient, args: &MetricsArgs) -> Result<Value> {
        let handler = MetricsHandler;

        let (from_ts, to_ts) = handler.parse_time_range(&args.from, &args.to)?;

        let response = client.query_metrics(&args.query, from_ts, to_ts).await?;

        let series = response.series.iter().map(|s| {
            let points_data = if let Some(ref pointlist) = s.pointlist {
                json!({
                    "count": pointlist.len(),
                    "data": pointlist.iter().map(|p| {
                        if p.len() >= 2 {
                            json!({
                                "timestamp": p[0].map(|t| crate::utils::format_timestamp(t as i64 / 1000))
                                    .unwrap_or_else(|| "N/A".to_string()),
                                "value": p[1]
                            })
                        } else {
                            json!({
                                "timestamp": "N/A",
                                "value": null
                            })
                        }
                    }).collect::<Vec<_>>()
                })
            } else {
                json!({
                    "count": 0,
                    "data": []
                })
            };

            // Build series object with only useful fields
            let mut series_obj = serde_json::Map::new();
            series_obj.insert("metric".to_string(), json!(s.metric));
            series_obj.insert("scope".to_string(), json!(s.scope));
            series_obj.insert("points".to_string(), points_data);

            // Add optional fields only if meaningful
            if let Some(ref aggr) = s.aggr {
                series_obj.insert("aggr".to_string(), json!(aggr));
            }
            if let Some(interval) = s.interval {
                series_obj.insert("interval".to_string(), json!(interval));
            }
            if let Some(ref unit) = s.unit {
                series_obj.insert("units".to_string(), json!(unit));
                if let Some(first_unit) = unit.iter().find(|u| u.is_some())
                    && let Some(u) = first_unit {
                        let mut unit_obj = serde_json::Map::new();
                        unit_obj.insert("name".to_string(), json!(u.name));
                        unit_obj.insert("family".to_string(), json!(u.family));
                        if let Some(ref short_name) = u.short_name
                            && !short_name.is_empty() {
                                unit_obj.insert("short_name".to_string(), json!(short_name));
                            }
                        series_obj.insert("unit".to_string(), json!(unit_obj));
                    }
            }

            json!(series_obj)
        }).collect::<Vec<_>>();

        // Build optimized meta - only include meaningful fields
        let mut meta = serde_json::Map::new();
        meta.insert("query".to_string(), json!(response.query));
        meta.insert("status".to_string(), json!(response.status));
        meta.insert(
            "from".to_string(),
            json!(crate::utils::format_timestamp(from_ts)),
        );
        meta.insert(
            "to".to_string(),
            json!(crate::utils::format_timestamp(to_ts)),
        );

        // Only include message if present and non-empty
        if let Some(ref message) = response.message
            && !message.is_empty()
        {
            meta.insert("message".to_string(), json!(message));
        }

        // Only include group_by if present and non-empty
        if let Some(ref group_by) = response.group_by
            && !group_by.is_empty()
        {
            meta.insert("group_by".to_string(), json!(group_by));
        }

        Ok(handler.format_list(json!(series), None, Some(json!(meta))))
    }

    pub async fn timeseries(client: &DatadogClient, args: &TimeseriesArgs) -> Result<Value> {
        let handler = MetricsHandler;

        let (from_ts, to_ts) = handler.parse_time_range(&args.from, &args.to)?;

        let interval = args
            .interval
            .map(|seconds| {
                seconds
                    .checked_mul(1000)
                    .filter(|&milliseconds| milliseconds > 0)
                    .ok_or_else(|| {
                        crate::error::DatadogError::InvalidInput(
                            "Interval must be positive and fit in milliseconds".into(),
                        )
                    })
            })
            .transpose()?;

        let response = client
            .query_timeseries(
                &args.queries,
                &args.formula,
                from_ts * 1000,
                to_ts * 1000,
                interval,
            )
            .await?;

        let series: Vec<Value> = response
            .series
            .iter()
            .zip(&response.values)
            .map(|(metadata, values)| {
                let points: Vec<Value> = response
                    .times
                    .iter()
                    .zip(values)
                    .map(|(time, value)| {
                        json!({
                            "timestamp": crate::utils::format_timestamp(time / 1000),
                            "value": value
                        })
                    })
                    .collect();
                let mut series = metadata.clone();
                series.insert(
                    "points".into(),
                    json!({"count": points.len(), "data": points}),
                );
                Value::Object(series)
            })
            .collect();

        let meta = json!({
            "queries": args.queries,
            "formulas": args.formula,
            "from": crate::utils::format_timestamp(from_ts),
            "to": crate::utils::format_timestamp(to_ts),
        });

        Ok(handler.format_list(json!(series), None, Some(meta)))
    }

    pub async fn scalar(client: &DatadogClient, args: &ScalarArgs) -> Result<Value> {
        let handler = MetricsHandler;

        let (from_ts, to_ts) = handler.parse_time_range(&args.from, &args.to)?;

        let response = client
            .query_scalar(
                &args.queries,
                &args.formula,
                &args.aggregator,
                from_ts * 1000,
                to_ts * 1000,
            )
            .await?;

        let meta = json!({
            "queries": args.queries,
            "formulas": args.formula,
            "aggregator": args.aggregator,
            "from": crate::utils::format_timestamp(from_ts),
            "to": crate::utils::format_timestamp(to_ts),
        });

        Ok(handler.format_list(serde_json::to_value(response)?, None, Some(meta)))
    }
}
