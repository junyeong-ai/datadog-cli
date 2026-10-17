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
                // Simplify unit - only include the first non-null unit
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

        // Only include error if present
        if let Some(ref error) = response.error
            && !error.is_empty()
        {
            meta.insert("error".to_string(), json!(error));
        }

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

        let attrs = &response["data"]["attributes"];
        let times = attrs["times"].as_array().cloned().unwrap_or_default();
        let empty = Vec::new();
        let series_meta = attrs["series"].as_array().unwrap_or(&empty);
        let values = attrs["values"].as_array().unwrap_or(&empty);

        let series: Vec<Value> = series_meta
            .iter()
            .zip(values.iter())
            .map(|(s, series_values)| {
                let points: Vec<Value> = times
                    .iter()
                    .zip(series_values.as_array().unwrap_or(&empty).iter())
                    .map(|(t, v)| {
                        json!({
                            "timestamp": t.as_i64()
                                .map(|ms| crate::utils::format_timestamp(ms / 1000)),
                            "value": v
                        })
                    })
                    .collect();

                let mut obj = serde_json::Map::new();
                for key in ["group_tags", "query_index", "unit"] {
                    if let Some(value) = s.get(key)
                        && !value.is_null()
                    {
                        obj.insert(key.to_string(), value.clone());
                    }
                }
                obj.insert(
                    "points".to_string(),
                    json!({ "count": points.len(), "data": points }),
                );
                json!(obj)
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

        // Columns are index-aligned; pivot them into one record per row so
        // the output works with jsonl and table formats.
        let empty = Vec::new();
        let columns = response["data"]["attributes"]["columns"]
            .as_array()
            .unwrap_or(&empty);

        let row_count = columns
            .iter()
            .filter_map(|c| c["values"].as_array().map(|v| v.len()))
            .max()
            .unwrap_or(0);

        let rows: Vec<Value> = (0..row_count)
            .map(|i| {
                let mut row = serde_json::Map::new();
                for column in columns {
                    let name = column["name"].as_str().unwrap_or("value");
                    row.insert(
                        name.to_string(),
                        column["values"].get(i).cloned().unwrap_or(Value::Null),
                    );
                }
                json!(row)
            })
            .collect();

        let meta = json!({
            "queries": args.queries,
            "formulas": args.formula,
            "aggregator": args.aggregator,
            "from": crate::utils::format_timestamp(from_ts),
            "to": crate::utils::format_timestamp(to_ts),
        });

        Ok(handler.format_list(json!(rows), None, Some(meta)))
    }
}
