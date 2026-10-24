use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use serde_json::{Value, json};

use crate::catalog::{Catalog, Operation, RetryPolicy};
use crate::datadog::{DatadogClient, QueryBudget, QueryCall, query_headers};
use crate::error::{DatadogError, Result};
use crate::schema::Violations;

mod time;
pub use time::apply_time_range;

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
    time_scope: Option<Value>,
    selection: Vec<String>,
    input: Value,
    max_pages: u32,
    wait: bool,
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
            time_scope: None,
            selection: Vec::new(),
            input: input.clone(),
            max_pages: 1,
            wait: false,
        })
    }

    pub fn with_time_scope(mut self, scope: Option<Value>) -> Self {
        self.time_scope = scope;
        self
    }

    pub fn with_selection(mut self, selection: &[String]) -> Result<Self> {
        if !selection.is_empty() && self.output.is_some() {
            return Err(DatadogError::InvalidInput(
                "Response selection cannot be combined with file output".into(),
            ));
        }
        for pointer in selection {
            if !pointer.is_empty() && !pointer.starts_with('/') {
                return Err(DatadogError::InvalidInput(
                    "--select requires JSON Pointers beginning with /, or an empty pointer".into(),
                ));
            }
            let mut characters = pointer.chars();
            while let Some(character) = characters.next() {
                if character == '~' && !matches!(characters.next(), Some('0' | '1')) {
                    return Err(DatadogError::InvalidInput(
                        "Invalid JSON Pointer escape in --select".into(),
                    ));
                }
            }
        }
        self.selection = selection.to_vec();
        Ok(self)
    }

    pub fn with_collection(mut self, max_pages: u32, wait: bool) -> Result<Self> {
        if !(1..=1000).contains(&max_pages) {
            return Err(DatadogError::InvalidInput(
                "Page limit must be between 1 and 1000".into(),
            ));
        }
        if max_pages > 1 {
            if self.operation.pagination.is_none() || self.output.is_some() || wait {
                return Err(DatadogError::InvalidInput("Multiple pages require a reviewed cursor contract and cannot use --output or --wait".into()));
            }
            if let Some(range) = &self.operation.time_range {
                for pointer in [&range.from, &range.to] {
                    let bound = self.input.pointer(pointer).ok_or_else(|| DatadogError::InvalidInput("Multiple pages require an explicit fixed time range; use --from and --to".into()))?;
                    if matches!(range.encoding, crate::catalog::TimeEncoding::Rfc3339)
                        && bound.as_str().is_none_or(|value| {
                            chrono::DateTime::parse_from_rfc3339(value).is_err()
                        })
                    {
                        return Err(DatadogError::InvalidInput("Multiple pages require absolute RFC3339 time bounds; use --from and --to".into()));
                    }
                }
            }
        }
        if wait
            && (!matches!(
                self.operation.id.as_str(),
                "v2.ExecuteDdsqlTabularQuery" | "v2.FetchDdsqlTabularQuery"
            ) || self.output.is_some())
        {
            return Err(DatadogError::InvalidInput(
                "--wait is supported only for DDSQL queries without --output".into(),
            ));
        }
        self.max_pages = max_pages;
        self.wait = wait;
        Ok(self)
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
        let mut current = Self::prepare(
            &crate::catalog::domain_id(&self.operation.domains[0]),
            &self.operation.id,
            &self.input,
            self.output.clone(),
        )?;
        let mut pages = Vec::new();
        let mut api_pages = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        if let Some(contract) = &self.operation.pagination
            && let Some(cursor) = self
                .input
                .pointer(&contract.request)
                .and_then(Value::as_str)
        {
            seen.insert(cursor.to_string());
        }
        let mut any_partial = false;
        let mut all_complete = true;
        let mut polls = 0_u32;
        let mut deviations = Violations::default();
        let (reply, body, next_cursor) = loop {
            let reply = client.execute_query(&current.call, &mut budget).await?;
            let body = current.decode(&reply)?;
            let processing = current.processing(&body);
            any_partial |= processing == "partial";
            all_complete &= processing == "complete";
            api_pages.push(body.get("meta").cloned().unwrap_or(Value::Null));
            if self.wait {
                let attributes = &body["data"]["attributes"];
                match attributes["state"].as_str() {
                    Some("completed") => {
                        if !attributes["columns"].is_array() {
                            return Err(DatadogError::DecodeError(
                                "Completed DDSQL response is missing columns".into(),
                            ));
                        }
                        current.record_deviations(&reply, &body, "", &mut deviations)?;
                        break (reply, body, None);
                    }
                    Some("running") => {
                        let id = attributes["query_id"]
                            .as_str()
                            .filter(|id| !id.is_empty())
                            .ok_or_else(|| {
                                DatadogError::DecodeError(
                                    "Running DDSQL response is missing query_id".into(),
                                )
                            })?;
                        current = Self::prepare(
                            "ddsql",
                            "v2.FetchDdsqlTabularQuery",
                            &json!({"body":{"data":{"type":"ddsql_query_fetch_request","attributes":{"query_id":id}}}}),
                            None,
                        )?;
                        polls += 1;
                        if budget.calls >= budget.max_calls {
                            return Err(DatadogError::QueryError(
                                "API call budget exhausted while DDSQL query is still running"
                                    .into(),
                            ));
                        }
                        tokio::time::timeout_at(
                            budget.deadline,
                            tokio::time::sleep(Duration::from_secs(1)),
                        )
                        .await
                        .map_err(|_| DatadogError::TimeoutError)?;
                        continue;
                    }
                    _ => {
                        return Err(DatadogError::DecodeError(
                            "Unknown DDSQL query state".into(),
                        ));
                    }
                }
            }
            let cursor = match &current.operation.pagination {
                Some(contract) => next_cursor(&body, &contract.response)?,
                None => None,
            };
            if self.max_pages > 1 {
                current.record_deviations(
                    &reply,
                    &body,
                    &format!("/pages/{}", pages.len()),
                    &mut deviations,
                )?;
                pages.push(self.project(&body)?);
                if let Some(cursor) = &cursor {
                    if !seen.insert(cursor.clone()) {
                        return Err(DatadogError::DecodeError(
                            "API repeated a pagination cursor".into(),
                        ));
                    }
                    if pages.len() < self.max_pages as usize {
                        let contract = self.operation.pagination.as_ref().ok_or_else(|| {
                            DatadogError::DecodeError("Missing cursor contract".into())
                        })?;
                        let mut input = current.input.clone();
                        time::set_pointer(&mut input, &contract.request, json!(cursor))?;
                        current = Self::prepare(
                            &crate::catalog::domain_id(&self.operation.domains[0]),
                            &self.operation.id,
                            &input,
                            None,
                        )?;
                        continue;
                    }
                }
            }
            if self.max_pages == 1 {
                current.record_deviations(&reply, &body, "", &mut deviations)?;
            }
            break (reply, body, cursor);
        };
        let response = current
            .operation
            .responses
            .get(&reply.status.to_string())
            .ok_or_else(|| DatadogError::DecodeError("Undeclared successful API status".into()))?;
        let media = response.media.as_deref();
        let api_meta = if api_pages.len() == 1 {
            api_pages.remove(0)
        } else {
            json!({"pages": api_pages})
        };
        let processing = if self.wait {
            current.processing(&body)
        } else if any_partial {
            "partial"
        } else if all_complete {
            "complete"
        } else {
            "unknown"
        };
        let data = if reply.status == 304 {
            Value::Null
        } else if let Some(path) = &self.output {
            write_download(path, &reply.bytes)?;
            json!({"path":path,"bytes":reply.bytes.len(),"media":media})
        } else if self.max_pages > 1 {
            json!({"pages":pages})
        } else {
            self.project(&body)?
        };
        Ok(json!({
            "data":data,
            "meta":{
                "api":api_meta,
                "source":{"operation":self.operation.id,"site":site,"path":self.operation.path,"catalog_commit":Catalog::load()?.source["commit"],"preview":self.operation.preview.is_some()},
                "http":{"status":reply.status,"headers":reply.headers},
                "execution":{"calls":budget.calls,"received_bytes":budget.received_bytes,"elapsed_ms":start.elapsed().as_millis()},
                "context":{"time_range":self.time_scope},
                "projection":{"pointers":self.selection},
                "job":{"wait":self.wait,"polls":polls},
                "result":{"schema_violations":deviations.entries(),"processing":processing,"collection":if self.max_pages == 1 { "single_response" } else if next_cursor.is_some() { "page_limit" } else { "exhausted" },"pages":if self.max_pages > 1 { pages.len() } else { 1 },"has_next":self.operation.pagination.as_ref().map(|_| next_cursor.is_some()),"next_cursor":next_cursor,"not_modified":reply.status==304}
            }
        }))
    }
    fn processing(&self, body: &Value) -> &'static str {
        self.operation
            .processing
            .as_ref()
            .and_then(|contract| {
                let status = body.pointer(&contract.pointer)?.as_str()?;
                if contract.complete.iter().any(|value| value == status) {
                    Some("complete")
                } else if contract.partial.iter().any(|value| value == status) {
                    Some("partial")
                } else {
                    None
                }
            })
            .unwrap_or("unknown")
    }

    /// Records where a returned JSON body deviates from the bundled response
    /// schema. The body is kept: the snapshot can trail the live API, and the
    /// fields this client interprets are checked where they are read.
    fn record_deviations(
        &self,
        reply: &crate::datadog::ApiReply,
        body: &Value,
        prefix: &str,
        deviations: &mut Violations,
    ) -> Result<()> {
        let Some(schema) = self
            .operation
            .responses
            .get(&reply.status.to_string())
            .filter(|response| response.media.as_deref().is_some_and(is_json))
            .and_then(|response| response.schema.as_ref())
        else {
            return Ok(());
        };
        let schema = Catalog::load()?.response_schema(self.operation, schema.clone())?;
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .offline()
            .build(&schema)
            .map_err(|error| {
                DatadogError::DecodeError(format!(
                    "Invalid bundled response schema for {} at {}",
                    self.operation.id,
                    error.schema_path()
                ))
            })?;
        deviations.record(&validator, body, prefix);
        Ok(())
    }

    fn project(&self, body: &Value) -> Result<Value> {
        if self.selection.is_empty() {
            return Ok(body.clone());
        }
        let mut fields = serde_json::Map::new();
        for pointer in &self.selection {
            let value = body.pointer(pointer).ok_or_else(|| {
                DatadogError::QueryError(format!(
                    "Selected JSON Pointer {pointer:?} is absent from the API response"
                ))
            })?;
            fields.insert(pointer.clone(), value.clone());
        }
        Ok(Value::Object(fields))
    }

    fn decode(&self, reply: &crate::datadog::ApiReply) -> Result<Value> {
        let response = self
            .operation
            .responses
            .get(&reply.status.to_string())
            .ok_or_else(|| DatadogError::DecodeError("Undeclared successful API status".into()))?;
        if response.media.as_deref().is_some_and(is_json) {
            let body: Value = serde_json::from_slice(&reply.bytes).map_err(|error| {
                DatadogError::DecodeError(format!(
                    "Invalid API JSON at line {}, column {}",
                    error.line(),
                    error.column()
                ))
            })?;
            crate::datadog::validate_query_processing(&self.operation.id, &body)?;
            if matches!(
                self.operation.id.as_str(),
                "v2.ExecuteDdsqlTabularQuery" | "v2.FetchDdsqlTabularQuery"
            ) {
                let attributes = &body["data"]["attributes"];
                match attributes["state"].as_str() {
                    Some("running")
                        if attributes["query_id"]
                            .as_str()
                            .is_some_and(|id| !id.is_empty()) => {}
                    Some("completed") if attributes["columns"].is_array() => {}
                    _ => {
                        return Err(DatadogError::DecodeError(
                            "DDSQL state is missing its required query_id or columns".into(),
                        ));
                    }
                }
                if let Some(warnings) = body.pointer("/data/attributes/warnings") {
                    crate::output::print_api_diagnostics(
                        &json!({"meta":{"api":{"warnings":warnings}}}),
                    )?;
                }
                if let Some(columns) = body
                    .pointer("/data/attributes/columns")
                    .and_then(Value::as_array)
                {
                    let expected = columns
                        .first()
                        .and_then(|column| column["values"].as_array())
                        .map(Vec::len);
                    if columns
                        .iter()
                        .any(|column| column["values"].as_array().map(Vec::len) != expected)
                    {
                        return Err(DatadogError::DecodeError(
                            "DDSQL column arrays have inconsistent row counts".into(),
                        ));
                    }
                }
            }
            Ok(body)
        } else if self.output.is_some() || (response.media.is_none() && reply.bytes.is_empty()) {
            Ok(Value::Null)
        } else {
            Err(DatadogError::DecodeError(
                "Unexpected non-JSON API response".into(),
            ))
        }
    }
}

/// Reads the reviewed cursor; an absent or null value ends collection, and any
/// other shape along its pointer is an error rather than a silent end.
fn next_cursor(body: &Value, pointer: &str) -> Result<Option<String>> {
    let mut value = body;
    for field in pointer.split('/').skip(1) {
        let field = field.replace("~1", "/").replace("~0", "~");
        value = match value {
            Value::Object(fields) => match fields.get(&field) {
                Some(value) => value,
                None => return Ok(None),
            },
            Value::Null => return Ok(None),
            _ => {
                return Err(DatadogError::DecodeError(
                    "API returned a malformed pagination cursor".into(),
                ));
            }
        };
    }
    match value {
        Value::Null => Ok(None),
        Value::String(cursor) => Ok((!cursor.is_empty()).then(|| cursor.clone())),
        _ => Err(DatadogError::DecodeError(
            "API returned a malformed pagination cursor".into(),
        )),
    }
}

fn is_json(media: &str) -> bool {
    let media = media.split(';').next().unwrap_or(media).trim();
    media == "application/json" || media.ends_with("+json")
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
