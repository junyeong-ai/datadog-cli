use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::{DatadogError, Result};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Access {
    Query,
    NotQuery,
    Deprecated,
    CredentialMaterial,
    RemoteAction,
    SeparateOrigin,
    Internal,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RetryPolicy {
    Read,
    Never,
}

#[derive(Debug, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub location: String,
    pub required: bool,
    pub description: String,
    pub style: String,
    pub explode: bool,
    pub schema: Value,
}

#[derive(Debug, Deserialize)]
pub struct ResponseContract {
    pub media: Option<String>,
    pub schema: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct Operation {
    pub id: String,
    pub operation_id: String,
    pub version: String,
    pub method: String,
    pub path: String,
    pub domains: Vec<String>,
    pub summary: String,
    pub description: String,
    pub preview: Option<String>,
    pub deprecated: bool,
    pub security: Vec<BTreeMap<String, Vec<String>>>,
    pub access: Access,
    pub reason: Option<String>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    #[serde(default)]
    pub body_required: bool,
    pub body_schema: Option<Value>,
    pub request_media: Option<String>,
    #[serde(default)]
    pub responses: BTreeMap<String, ResponseContract>,
    pub retry: Option<RetryPolicy>,
    pub pagination: Option<Value>,
    pub servers: Option<Value>,
}

pub struct Catalog {
    pub source: Value,
    pub operations: BTreeMap<String, Operation>,
    definitions: BTreeMap<String, Value>,
}

pub fn domain_id(name: &str) -> String {
    name.to_ascii_lowercase().replace(' ', "-")
}

impl Catalog {
    pub fn load() -> Result<&'static Self> {
        static CATALOG: OnceLock<std::result::Result<Catalog, String>> = OnceLock::new();
        CATALOG
            .get_or_init(|| {
                (|| -> std::result::Result<Self, serde_json::Error> {
                    Ok(Self {
                        source: serde_json::from_str(include_str!("../api/source.json"))?,
                        operations: serde_json::from_str(include_str!("../api/catalog.json"))?,
                        definitions: BTreeMap::from([
                            (
                                "v1".into(),
                                serde_json::from_str(include_str!("../api/v1.schemas.json"))?,
                            ),
                            (
                                "v2".into(),
                                serde_json::from_str(include_str!("../api/v2.schemas.json"))?,
                            ),
                        ]),
                    })
                })()
                .map_err(|error| error.to_string())
            })
            .as_ref()
            .map_err(|error| {
                DatadogError::DecodeError(format!("Invalid bundled API catalog: {error}"))
            })
    }

    pub fn operation(&self, id: &str) -> Result<&Operation> {
        self.operations.get(id).ok_or_else(|| {
            DatadogError::InvalidInput(format!(
                "Unknown operation {id:?}; use capabilities list --domain DOMAIN"
            ))
        })
    }

    pub fn query(&self, domain: &str, id: &str) -> Result<&Operation> {
        let operation = self.operation(id)?;
        if operation.access != Access::Query {
            return Err(DatadogError::InvalidInput(format!(
                "Operation {id} is unavailable for queries: {}",
                operation
                    .reason
                    .as_deref()
                    .unwrap_or("not an approved query")
            )));
        }
        if !operation
            .domains
            .iter()
            .any(|name| domain_id(name) == domain)
        {
            return Err(DatadogError::InvalidInput(format!(
                "Operation {id} does not belong to domain {domain:?}"
            )));
        }
        Ok(operation)
    }

    pub fn domains(&self, include_excluded: bool) -> Value {
        let mut domains = BTreeMap::<String, Value>::new();
        for operation in self.operations.values() {
            if !include_excluded && operation.access != Access::Query {
                continue;
            }
            for name in &operation.domains {
                let entry = domains.entry(domain_id(name)).or_insert_with(
                    || json!({"name":name,"operations":0,"queries":0,"preview_queries":0}),
                );
                entry["operations"] = json!(entry["operations"].as_u64().unwrap_or(0) + 1);
                if operation.access == Access::Query {
                    entry["queries"] = json!(entry["queries"].as_u64().unwrap_or(0) + 1);
                    if operation.preview.is_some() {
                        entry["preview_queries"] =
                            json!(entry["preview_queries"].as_u64().unwrap_or(0) + 1);
                    }
                }
            }
        }
        json!({"data":domains,"meta":{"catalog_source":self.source,"availability":"Not probed; product and user permissions are required."}})
    }

    pub fn list(&self, domain: Option<&str>, include_excluded: bool) -> Result<Value> {
        if let Some(domain) = domain
            && !self.operations.values().any(|operation| {
                operation
                    .domains
                    .iter()
                    .any(|name| domain_id(name) == domain)
            })
        {
            return Err(DatadogError::InvalidInput(format!(
                "Unknown domain {domain:?}"
            )));
        }
        let operations = self
            .operations
            .values()
            .filter(|operation| {
                (include_excluded || operation.access == Access::Query)
                    && domain.is_none_or(|domain| {
                        operation
                            .domains
                            .iter()
                            .any(|name| domain_id(name) == domain)
                    })
            })
            .map(|operation| self.summary(operation))
            .collect::<Vec<_>>();
        Ok(json!({"data":operations,"meta":{"catalog_source":self.source}}))
    }

    fn summary(&self, operation: &Operation) -> Value {
        json!({
            "id":operation.id,"domains":operation.domains.iter().map(|name| domain_id(name)).collect::<Vec<_>>(),
            "summary":operation.summary,"method":operation.method,"path":operation.path,
            "preview":operation.preview.is_some(),"deprecated":operation.deprecated,
            "access":operation.access,"reason":operation.reason,
            "security":operation.security,
        })
    }

    pub fn describe(&self, id: &str, include_response_schema: bool) -> Result<Value> {
        let operation = self.operation(id)?;
        let mut description = self.summary(operation);
        description["description"] = json!(operation.description);
        description["preview_notice"] = json!(operation.preview);
        if operation.access == Access::Query {
            description["input_schema"] = self.input_schema(operation)?;
            let mut responses = BTreeMap::new();
            for (status, response) in &operation.responses {
                let mut contract = json!({"media":response.media});
                if include_response_schema && let Some(schema) = &response.schema {
                    contract["schema"] = self.with_definitions(operation, schema.clone())?;
                }
                responses.insert(status.clone(), contract);
            }
            description["responses"] = json!(responses);
            description["retry"] = json!(operation.retry);
            description["pagination"] = json!(operation.pagination);
        }
        Ok(json!({"data":description,"meta":{"catalog_source":self.source}}))
    }

    pub fn input_schema(&self, operation: &Operation) -> Result<Value> {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        for (input, location) in [("path", "path"), ("query", "query"), ("headers", "header")] {
            let mut fields = serde_json::Map::new();
            let mut field_required = Vec::new();
            for parameter in operation
                .parameters
                .iter()
                .filter(|parameter| parameter.location == location)
            {
                let mut field = parameter.schema.clone();
                field["description"] = json!(parameter.description);
                fields.insert(parameter.name.clone(), field);
                if parameter.required {
                    field_required.push(parameter.name.clone());
                }
            }
            if !fields.is_empty() {
                if !field_required.is_empty() {
                    required.push(input);
                }
                properties.insert(input.into(), json!({"type":"object","properties":fields,"required":field_required,"additionalProperties":false}));
            }
        }
        if let Some(schema) = &operation.body_schema {
            properties.insert("body".into(), schema.clone());
            if operation.body_required {
                required.push("body");
            }
        }
        self.with_definitions(operation, json!({"$schema":"http://json-schema.org/draft-07/schema#","type":"object","properties":properties,"required":required,"additionalProperties":false}))
    }

    pub fn with_definitions(&self, operation: &Operation, mut schema: Value) -> Result<Value> {
        let definitions = self.definitions.get(&operation.version).ok_or_else(|| {
            DatadogError::DecodeError("Unknown bundled API schema version".into())
        })?;
        let mut needed = BTreeSet::new();
        let mut pending = Vec::new();
        references(&schema, &mut pending);
        while let Some(name) = pending.pop() {
            if needed.insert(name.clone()) {
                let definition = definitions.get(&name).ok_or_else(|| {
                    DatadogError::DecodeError(format!("Missing bundled API definition {name}"))
                })?;
                references(definition, &mut pending);
            }
        }
        schema["definitions"] = Value::Object(
            needed
                .into_iter()
                .map(|name| (name.clone(), definitions[&name].clone()))
                .collect(),
        );
        Ok(schema)
    }

    pub fn validate_input(&self, operation: &Operation, input: &Value) -> Result<()> {
        let schema = self.input_schema(operation)?;
        let validator = jsonschema::options()
            .offline()
            .should_validate_formats(true)
            .build(&schema)
            .map_err(|error| {
                DatadogError::DecodeError(format!(
                    "Invalid bundled input schema for {} at {}",
                    operation.id,
                    error.schema_path()
                ))
            })?;
        if let Some(error) = validator.iter_errors(input).next() {
            return Err(DatadogError::InvalidInput(format!(
                "Input for {} violates the API schema at {} (constraint {})",
                operation.id,
                error.instance_path(),
                error.schema_path()
            )));
        }
        Ok(())
    }
}

fn references(value: &Value, output: &mut Vec<String>) {
    let Some(fields) = value.as_object() else {
        return;
    };
    if let Some(name) = fields
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|value| value.strip_prefix("#/definitions/"))
    {
        output.push(name.to_string());
    }
    for keyword in ["properties", "patternProperties", "definitions", "$defs"] {
        if let Some(values) = fields.get(keyword).and_then(Value::as_object) {
            values.values().for_each(|value| references(value, output));
        }
    }
    for keyword in [
        "items",
        "additionalProperties",
        "not",
        "contains",
        "propertyNames",
    ] {
        if let Some(value) = fields.get(keyword) {
            references(value, output);
        }
    }
    for keyword in ["allOf", "oneOf", "anyOf"] {
        if let Some(values) = fields.get(keyword).and_then(Value::as_array) {
            values.iter().for_each(|value| references(value, output));
        }
    }
}
