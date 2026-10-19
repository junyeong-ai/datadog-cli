use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use serde_json::{Value, json};

use crate::catalog::{Catalog, Operation, RetryPolicy};
use crate::datadog::{DatadogClient, QueryBudget, QueryCall, query_headers};
use crate::error::{DatadogError, Result};

const PATH_COMPONENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');
pub const MAX_INPUT_BYTES: u64 = 2 * 1024 * 1024;

pub struct PreparedQuery {
    operation: &'static Operation,
    call: QueryCall,
    output: Option<PathBuf>,
}

pub async fn read_input(argument: Option<&str>) -> Result<Value> {
    let argument = argument.unwrap_or("{}").to_string();
    let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        if argument == "-" {
            std::io::stdin()
                .take(MAX_INPUT_BYTES + 1)
                .read_to_end(&mut bytes)?;
        } else if let Some(path) = argument.strip_prefix('@') {
            std::fs::File::open(path)?
                .take(MAX_INPUT_BYTES + 1)
                .read_to_end(&mut bytes)?;
        } else {
            bytes = argument.into_bytes();
        }
        if bytes.len() as u64 > MAX_INPUT_BYTES {
            return Err(DatadogError::InvalidInput(format!(
                "Input exceeds the {MAX_INPUT_BYTES} byte limit"
            )));
        }
        Ok(bytes)
    })
    .await
    .map_err(|_| DatadogError::IoError(std::io::Error::other("Input reader failed")))??;
    serde_json::from_slice(&bytes).map_err(|error| {
        DatadogError::InvalidInput(format!(
            "Invalid input JSON at line {}, column {}",
            error.line(),
            error.column()
        ))
    })
}

impl PreparedQuery {
    pub fn prepare(domain: &str, id: &str, input: &Value, output: Option<PathBuf>) -> Result<Self> {
        let catalog = Catalog::load()?;
        let operation = catalog.query(domain, id)?;
        catalog.validate_input(operation, input)?;
        let mut endpoint = operation.path.clone();
        let mut parameters = Vec::new();
        let mut headers = Vec::new();
        for parameter in &operation.parameters {
            let container = match parameter.location.as_str() {
                "path" => "path",
                "query" => "query",
                "header" => "headers",
                _ => {
                    return Err(DatadogError::DecodeError(
                        "Unsupported bundled parameter location".into(),
                    ));
                }
            };
            let Some(value) = input
                .get(container)
                .and_then(|fields| fields.get(&parameter.name))
            else {
                continue;
            };
            match parameter.location.as_str() {
                "path" => {
                    let value = scalar(value)?;
                    if value == "." || value == ".." {
                        return Err(DatadogError::InvalidInput(
                            "Path parameters cannot be dot segments".into(),
                        ));
                    }
                    let value = utf8_percent_encode(&value, PATH_COMPONENT).to_string();
                    endpoint = endpoint.replace(&format!("{{{}}}", parameter.name), &value);
                }
                "query" => parameters.extend(serialize_parameter(
                    &parameter.name,
                    value,
                    parameter.explode,
                )?),
                "header" => headers.push((parameter.name.clone(), scalar(value)?)),
                _ => unreachable!(),
            }
        }
        if endpoint.contains('{') || endpoint.contains('}') {
            return Err(DatadogError::DecodeError(
                "Bundled path parameters are incomplete".into(),
            ));
        }
        if operation.servers.is_some() {
            return Err(DatadogError::InvalidInput(
                "Queries cannot change the authenticated API origin".into(),
            ));
        }
        if output.is_none()
            && operation.responses.values().any(|response| {
                response
                    .media
                    .as_deref()
                    .is_some_and(|media| !is_json(media))
            })
        {
            return Err(DatadogError::InvalidInput(
                "This API returns a file; provide --output PATH".into(),
            ));
        }
        if let Some(path) = &output
            && path.exists()
        {
            return Err(DatadogError::InvalidInput(
                "Output file already exists".into(),
            ));
        }
        let accepted_statuses = operation
            .responses
            .keys()
            .map(|status| {
                status.parse::<u16>().map_err(|_| {
                    DatadogError::DecodeError("Invalid bundled response status".into())
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let accept = operation
            .responses
            .values()
            .filter_map(|response| response.media.as_deref())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        let call = QueryCall {
            method: reqwest::Method::from_bytes(operation.method.as_bytes())
                .map_err(|_| DatadogError::DecodeError("Invalid bundled HTTP method".into()))?,
            endpoint,
            parameters,
            headers: query_headers(headers.into_iter())?,
            body: input.get("body").cloned(),
            content_type: operation
                .request_media
                .clone()
                .unwrap_or_else(|| "application/json".into()),
            accept,
            retry: operation.retry.unwrap_or(RetryPolicy::Never),
            accepted_statuses,
        };
        Ok(Self {
            operation,
            call,
            output,
        })
    }

    pub async fn execute(
        &self,
        client: &DatadogClient,
        site: &str,
        duration: Duration,
        max_bytes: usize,
        max_calls: u32,
    ) -> Result<Value> {
        if duration.is_zero()
            || duration > Duration::from_secs(3600)
            || max_bytes == 0
            || max_bytes > 512 * 1024 * 1024
            || max_calls == 0
            || max_calls > 1000
        {
            return Err(DatadogError::InvalidInput(
                "Query execution limits are outside their supported ranges".into(),
            ));
        }
        let start = Instant::now();
        let mut budget = QueryBudget::new(duration, max_bytes, max_calls);
        let reply = client.execute_query(&self.call, &mut budget).await?;
        let response = self
            .operation
            .responses
            .get(&reply.status.to_string())
            .ok_or_else(|| DatadogError::DecodeError("Undeclared successful API status".into()))?;
        let media = response.media.as_deref();
        let body = if media.is_some_and(is_json) {
            let body: Value = serde_json::from_slice(&reply.bytes).map_err(|error| {
                DatadogError::DecodeError(format!(
                    "Invalid API JSON at line {}, column {}",
                    error.line(),
                    error.column()
                ))
            })?;
            crate::datadog::validate_query_processing(&self.operation.id, &body)?;
            if let Some(schema) = &response.schema {
                validate_response(self.operation, schema, &body)?;
            }
            body
        } else if self.output.is_some() || (media.is_none() && reply.bytes.is_empty()) {
            Value::Null
        } else {
            return Err(DatadogError::DecodeError(
                "Unexpected non-JSON API response".into(),
            ));
        };
        let api_meta = body.get("meta").cloned();
        let data = if reply.status == 304 {
            Value::Null
        } else if let Some(path) = &self.output {
            write_download(path, &reply.bytes)?;
            json!({"path":path,"bytes":reply.bytes.len(),"media":media})
        } else {
            body
        };
        Ok(json!({
            "data":data,
            "meta":{
                "api":api_meta,
                "source":{"operation":self.operation.id,"site":site,"path":self.operation.path,"catalog_commit":Catalog::load()?.source["commit"],"preview":self.operation.preview.is_some()},
                "http":{"status":reply.status,"headers":reply.headers},
                "execution":{"calls":budget.calls,"received_bytes":budget.received_bytes,"elapsed_ms":start.elapsed().as_millis()},
                "result":{"processing":"unknown","collection":"single_response","not_modified":reply.status==304}
            }
        }))
    }
}

fn is_json(media: &str) -> bool {
    let media = media.split(';').next().unwrap_or(media).trim();
    media == "application/json" || media.ends_with("+json")
}

fn validate_response(operation: &Operation, schema: &Value, body: &Value) -> Result<()> {
    let schema = Catalog::load()?.with_definitions(operation, schema.clone())?;
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .offline()
        .build(&schema)
        .map_err(|error| {
            DatadogError::DecodeError(format!(
                "Invalid bundled response schema for {} at {}",
                operation.id,
                error.schema_path()
            ))
        })?;
    if let Some(error) = validator.iter_errors(body).next() {
        return Err(DatadogError::DecodeError(format!(
            "Response for {} violates the API schema at {} (constraint {})",
            operation.id,
            error.instance_path(),
            error.schema_path()
        )));
    }
    Ok(())
}

fn scalar(value: &Value) -> Result<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        _ => Err(DatadogError::InvalidInput(
            "URI parameters must contain scalar values".into(),
        )),
    }
}

fn serialize_parameter(name: &str, value: &Value, explode: bool) -> Result<Vec<(String, String)>> {
    match value {
        Value::Array(values) => {
            let values = values.iter().map(scalar).collect::<Result<Vec<_>>>()?;
            if explode {
                Ok(values
                    .into_iter()
                    .map(|value| (name.to_string(), value))
                    .collect())
            } else {
                Ok(vec![(name.to_string(), values.join(","))])
            }
        }
        Value::Object(values) => {
            let values = values
                .iter()
                .map(|(key, value)| Ok((key.clone(), scalar(value)?)))
                .collect::<Result<BTreeMap<_, _>>>()?;
            if explode {
                Ok(values.into_iter().collect())
            } else {
                Ok(vec![(
                    name.to_string(),
                    values
                        .into_iter()
                        .flat_map(|(key, value)| [key, value])
                        .collect::<Vec<_>>()
                        .join(","),
                )])
            }
        }
        _ => Ok(vec![(name.to_string(), scalar(value)?)]),
    }
}

fn write_download(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path)
        .map_err(|error| DatadogError::IoError(error.error))?;
    Ok(())
}
