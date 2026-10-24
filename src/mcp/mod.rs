use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use rmcp::{
    ServiceExt,
    model::{
        CallToolRequestParams, CallToolResponse, ClientCapabilities, ClientConfig, Implementation,
        PaginatedRequestParams, ProtocolVersion,
    },
    transport::streamable_http_client::{
        StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    config::Config,
    error::{DatadogError, Result},
};
mod transport;

#[derive(Deserialize)]
struct ReviewedTool {
    toolset: String,
    description: String,
}

fn policy() -> Result<&'static BTreeMap<String, ReviewedTool>> {
    static TOOLS: OnceLock<std::result::Result<BTreeMap<String, ReviewedTool>, String>> =
        OnceLock::new();
    TOOLS
        .get_or_init(|| {
            serde_json::from_str(include_str!("../../api/mcp-tools.json"))
                .map_err(|_| "Invalid bundled MCP policy".into())
        })
        .as_ref()
        .map_err(|error| DatadogError::DecodeError(error.clone()))
}

pub fn list(
    toolset: Option<&str>,
    search: Option<&str>,
    offset: usize,
    limit: usize,
) -> Result<Value> {
    if !(1..=200).contains(&limit) {
        return Err(DatadogError::InvalidInput(
            "MCP discovery limit must be between 1 and 200".into(),
        ));
    }
    let tools = policy()?;
    if let Some(toolset) = toolset
        && !tools.values().any(|tool| tool.toolset == toolset)
    {
        let reviewed = tools
            .values()
            .map(|tool| tool.toolset.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        return Err(DatadogError::InvalidInput(format!(
            "Unknown reviewed MCP toolset {toolset:?}; reviewed toolsets: {}",
            reviewed.join(", ")
        )));
    }
    let search = search.map(str::to_ascii_lowercase);
    let items = tools
        .iter()
        .filter(|(name, tool)| {
            toolset.is_none_or(|selected| selected == tool.toolset)
                && search.as_ref().is_none_or(|query| {
                    name.to_ascii_lowercase().contains(query)
                        || tool.description.to_ascii_lowercase().contains(query)
                })
        })
        .collect::<Vec<_>>();
    let total = items.len();
    let data=items.into_iter().skip(offset).take(limit).map(|(name,tool)|json!({"name":name,"toolset":tool.toolset,"description":tool.description})).collect::<Vec<_>>();
    let next = offset.saturating_add(data.len());
    Ok(
        json!({"data":data,"meta":{"source":serde_json::from_str::<Value>(include_str!("../../api/mcp-source.json"))?,"pagination":{"total_count":total,"returned":data.len(),"has_next":next<total,"next_offset":if next<total {Some(next)}else{None},"has_next_is_estimate":false}}}),
    )
}

pub fn endpoint(site: &str, toolset: &str) -> Result<String> {
    let host = match site {
        "datadoghq.com" => "mcp.datadoghq.com",
        "us3.datadoghq.com" => "mcp.us3.datadoghq.com",
        "us5.datadoghq.com" => "mcp.us5.datadoghq.com",
        "datadoghq.eu" => "mcp.datadoghq.eu",
        "ap1.datadoghq.com" => "mcp.ap1.datadoghq.com",
        "ap2.datadoghq.com" => "mcp.ap2.datadoghq.com",
        "uk1.datadoghq.com" => "mcp.uk1.datadoghq.com",
        _ => {
            return Err(DatadogError::InvalidInput(
                "The official Datadog MCP server is not supported on this site".into(),
            ));
        }
    };
    if !policy()?.values().any(|tool| tool.toolset == toolset) {
        return Err(DatadogError::InvalidInput(
            "Unknown reviewed MCP toolset".into(),
        ));
    }
    Ok(format!("https://{host}/v1/mcp?toolsets={toolset}"))
}

pub struct PreparedMcp {
    name: String,
    toolset: &'static str,
    input: Option<serde_json::Map<String, Value>>,
}

impl PreparedMcp {
    pub fn prepare(name: &str, input: Option<Value>) -> Result<Self> {
        let reviewed = policy()?.get(name).ok_or_else(|| {
            DatadogError::InvalidInput(
                "MCP tool is not an individually reviewed read/analysis tool; use mcp list".into(),
            )
        })?;
        let input = input
            .map(|value| {
                value.as_object().cloned().ok_or_else(|| {
                    DatadogError::InvalidInput("MCP arguments must be a JSON object".into())
                })
            })
            .transpose()?;
        Ok(Self {
            name: name.to_string(),
            toolset: &reviewed.toolset,
            input,
        })
    }

    pub fn check_site(&self, site: &str) -> Result<()> {
        endpoint(site, self.toolset).map(|_| ())
    }

    pub async fn execute(
        &self,
        config: &Config,
        duration: Duration,
        max_calls: u32,
        max_bytes: usize,
    ) -> Result<Value> {
        let uri = endpoint(&config.site, self.toolset)?;
        self.execute_at(config, uri, duration, max_calls, max_bytes)
            .await
    }

    #[doc(hidden)]
    pub async fn execute_at(
        &self,
        config: &Config,
        uri: String,
        duration: Duration,
        max_calls: u32,
        max_bytes: usize,
    ) -> Result<Value> {
        if duration.is_zero()
            || duration > Duration::from_secs(3600)
            || !(1..=1000).contains(&max_calls)
            || !(1..=512 * 1024 * 1024).contains(&max_bytes)
        {
            return Err(DatadogError::InvalidInput(
                "MCP execution limits are outside their supported ranges".into(),
            ));
        }
        let started = Instant::now();
        let backend =
            transport::BoundedHttp::new(config, uri.clone(), duration, max_calls, max_bytes)?;
        let mut transport_config = StreamableHttpClientTransportConfig::with_uri(uri.clone())
            .max_concurrent_requests(1)
            .reinit_on_expired_session(false)
            .max_sse_event_size(max_bytes);
        transport_config.retry_config = Arc::new(transport::NoReconnect);
        let transport =
            StreamableHttpClientTransport::with_client(backend.clone(), transport_config);
        let future = async {
            let client = ClientConfig::new(
                ClientCapabilities::default(),
                Implementation::new("datadog-cli", env!("CARGO_PKG_VERSION")),
            )
            .with_protocol_version(ProtocolVersion::V_2025_11_25)
            .serve(transport)
            .await
            .map_err(|error| backend.error(initialization_error(error)))?;
            let protocol = client.peer_info().map(|info| info.protocol_version.clone());
            let result = self.query_tool(&client, &backend).await;
            let cleanup = client.cancel().await;
            let data = result?;
            cleanup.map_err(|_| {
                backend.error(DatadogError::QueryError(
                    "MCP session cleanup failed".into(),
                ))
            })?;
            let (calls, bytes) = backend.usage()?;
            if let Some(error) = backend.take_failure() {
                return Err(error);
            }
            Ok(json!({
                "data": data,
                "meta": {
                    "source": {
                        "backend": "mcp", "tool": self.name, "toolset": self.toolset,
                        "site": config.site, "endpoint": uri, "protocol_version": protocol,
                        "policy": serde_json::from_str::<Value>(include_str!("../../api/mcp-source.json"))?
                    },
                    "execution": {"calls": calls, "received_bytes": bytes, "elapsed_ms": started.elapsed().as_millis()},
                    "result": {"processing": "unknown", "collection": "tool_response"}
                }
            }))
        };
        tokio::time::timeout(duration, future)
            .await
            .map_err(|_| DatadogError::TimeoutError)?
    }

    async fn query_tool(
        &self,
        client: &rmcp::service::RunningService<rmcp::RoleClient, ClientConfig>,
        backend: &transport::BoundedHttp,
    ) -> Result<Value> {
        let mut cursor = None;
        let mut cursors = BTreeSet::new();
        let mut selected = None;
        let mut advertised = BTreeSet::new();
        loop {
            let page = client
                .list_tools(
                    cursor
                        .take()
                        .map(|cursor| PaginatedRequestParams::default().with_cursor(Some(cursor))),
                )
                .await
                .map_err(|error| backend.error(service_error("tool discovery", error)))?;
            for tool in page.tools {
                if !advertised.insert(tool.name.to_string()) {
                    return Err(DatadogError::DecodeError(
                        "MCP discovery returned a duplicate tool name".into(),
                    ));
                }
                if tool.name == self.name && selected.replace(tool).is_some() {
                    return Err(DatadogError::DecodeError(
                        "MCP discovery returned a duplicate tool name".into(),
                    ));
                }
            }
            match page.next_cursor {
                Some(next) if !next.is_empty() => {
                    if !cursors.insert(next.clone()) {
                        return Err(DatadogError::DecodeError(
                            "MCP discovery repeated a pagination cursor".into(),
                        ));
                    }
                    cursor = Some(next);
                }
                _ => break,
            }
        }
        let tool = selected.ok_or_else(|| {
            DatadogError::QueryError(
                format!("The reviewed tool is unavailable from this MCP server or identity. Advertised tools (up to 16): {}", advertised.iter().take(16).cloned().collect::<Vec<_>>().join(", ")),
            )
        })?;
        if self.input.is_none() {
            return serde_json::to_value(tool).map_err(Into::into);
        }
        if tool
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.read_only_hint)
            == Some(false)
        {
            return Err(DatadogError::QueryError(
                "MCP server marks this reviewed tool as mutating; execution is refused".into(),
            ));
        }
        let input = self.input.as_ref().ok_or_else(|| {
            DatadogError::InvalidInput("MCP arguments are required for a tool call".into())
        })?;
        validate_arguments(
            &Value::Object((*tool.input_schema).clone()),
            &Value::Object(input.clone()),
        )?;
        let result = client
            .call_tool_once(
                CallToolRequestParams::new(self.name.clone()).with_arguments(input.clone()),
            )
            .await
            .map_err(|error| backend.error(service_error("tool call", error)))?;
        match result {
            CallToolResponse::Complete(result) => {
                if result.is_error == Some(true) {
                    return Err(DatadogError::QueryError(format!(
                        "MCP tool {} reported an error: {}",
                        self.name,
                        serde_json::to_string(&result.content)?
                    )));
                }
                if let Some(schema) = &tool.output_schema {
                    let content = result.structured_content.as_ref().ok_or_else(|| {
                        DatadogError::DecodeError(
                            "MCP tool declared an output schema but returned no structured content"
                                .into(),
                        )
                    })?;
                    validate_result(&Value::Object((**schema).clone()), content)?;
                }
                serde_json::to_value(result).map_err(Into::into)
            }
            CallToolResponse::InputRequired(_) => Err(DatadogError::QueryError(
                "MCP tool requested interactive input; no follow-up call was performed".into(),
            )),
            _ => Err(DatadogError::DecodeError(
                "Unsupported MCP tool response type".into(),
            )),
        }
    }
}

fn validate_arguments(schema: &Value, input: &Value) -> Result<()> {
    let validator = jsonschema::options()
        .offline()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|_| DatadogError::DecodeError("Unsupported MCP input schema".into()))?;
    if let Some(violations) = crate::schema::violations(&validator, input) {
        return Err(DatadogError::InvalidInput(format!(
            "MCP arguments violate the tool schema: {violations}"
        )));
    }
    Ok(())
}

fn validate_result(schema: &Value, result: &Value) -> Result<()> {
    let validator = jsonschema::options()
        .offline()
        .build(schema)
        .map_err(|_| DatadogError::DecodeError("Unsupported MCP output schema".into()))?;
    if let Some(violations) = crate::schema::violations(&validator, result) {
        return Err(DatadogError::DecodeError(format!(
            "MCP output violates the tool schema: {violations}"
        )));
    }
    Ok(())
}

fn initialization_error(error: rmcp::service::ClientInitializeError) -> DatadogError {
    use rmcp::service::ClientInitializeError;
    let detail = match error {
        ClientInitializeError::JsonRpcError(error) => {
            format!("JSON-RPC code {}: {}", error.code.0, error.message)
        }
        ClientInitializeError::NoCompatibleProtocolVersion {
            client_supported,
            server_supported,
        } => format!(
            "incompatible protocol versions (client {client_supported:?}, server {server_supported:?})"
        ),
        ClientInitializeError::ConnectionClosed(context) => {
            format!("connection closed while {context}")
        }
        ClientInitializeError::TransportError { context, .. } => {
            format!("transport failed while {context}")
        }
        ClientInitializeError::ExpectedInitResponse(_) => {
            "server did not return an initialize response".into()
        }
        ClientInitializeError::ExpectedInitResult(_) => {
            "server did not return an initialize result".into()
        }
        _ => "invalid initialization response".into(),
    };
    DatadogError::DecodeError(format!("MCP initialization failed: {detail}"))
}

fn service_error(context: &str, error: rmcp::service::ServiceError) -> DatadogError {
    use rmcp::service::ServiceError;
    match error {
        ServiceError::McpError(error) => DatadogError::QueryError(format!(
            "MCP {context} failed (JSON-RPC code {}): {}",
            error.code.0, error.message
        )),
        ServiceError::Timeout { .. } => DatadogError::TimeoutError,
        _ => DatadogError::QueryError(format!("MCP {context} failed")),
    }
}
