use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::{DatadogError, Result};
use crate::pagination::PaginationInfo;

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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeEncoding {
    UnixSeconds,
    UnixMilliseconds,
    Rfc3339,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TimeRangeContract {
    pub from: String,
    pub to: String,
    pub encoding: TimeEncoding,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProcessingContract {
    pub pointer: String,
    pub complete: Vec<String>,
    pub partial: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PaginationContract {
    pub request: String,
    pub response: String,
}

#[derive(Debug, Deserialize)]
pub struct ExclusiveInput {
    pub field: String,
    pub conflicts: Vec<String>,
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
    pub pagination: Option<PaginationContract>,
    pub servers: Option<Value>,
    pub time_range: Option<TimeRangeContract>,
    pub processing: Option<ProcessingContract>,
    #[serde(default)]
    pub exclusive_input: Vec<ExclusiveInput>,
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
            let domains = operation
                .domains
                .iter()
                .map(|name| format!("{:?}", domain_id(name)))
                .collect::<Vec<_>>();
            return Err(DatadogError::InvalidInput(format!(
                "Operation {id} does not belong to domain {domain:?}; its domains: {}",
                domains.join(", ")
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
        let data = domains
            .into_iter()
            .map(|(id, mut value)| {
                value["id"] = json!(id);
                value
            })
            .collect::<Vec<_>>();
        json!({"data":data,"meta":{"catalog_source":self.source,"availability":"Not probed; product and user permissions are required."}})
    }

    pub fn list(
        &self,
        domain: Option<&str>,
        include_excluded: bool,
        search: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Result<Value> {
        if !(1..=200).contains(&limit) {
            return Err(DatadogError::InvalidInput(
                "Capability page size must be between 1 and 200".into(),
            ));
        }
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
        let search = search.map(str::to_ascii_lowercase);
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
            .filter(|operation| {
                search.as_ref().is_none_or(|search| {
                    operation.id.to_ascii_lowercase().contains(search)
                        || operation.summary.to_ascii_lowercase().contains(search)
                })
            })
            .collect::<Vec<_>>();
        let total = operations.len();
        let data = operations
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|operation| self.summary(operation))
            .collect::<Vec<_>>();
        let pagination = PaginationInfo::from_offset(data.len(), offset, limit, Some(total));
        Ok(json!({"data":data,"pagination":pagination,"meta":{"catalog_source":self.source}}))
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
                    contract["schema"] = self.response_schema(operation, schema.clone())?;
                }
                responses.insert(status.clone(), contract);
            }
            description["responses"] = json!(responses);
            description["retry"] = json!(operation.retry);
            description["pagination"] = json!(operation.pagination);
            description["time_range"] = json!(operation.time_range);
            description["processing"] = json!(operation.processing);
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
        let mut schema = self.with_definitions(operation, json!({"$schema":"http://json-schema.org/draft-07/schema#","type":"object","properties":properties,"required":required,"additionalProperties":false}))?;
        apply_schema_context(
            &mut schema,
            SchemaContext::Request,
            &self.definitions[&operation.version],
        );
        if !operation.exclusive_input.is_empty() {
            schema["allOf"] = Value::Array(operation.exclusive_input.iter().map(|rule| {
                json!({"not":{"allOf":[presence_schema(&rule.field),{"anyOf":rule.conflicts.iter().map(|pointer|presence_schema(pointer)).collect::<Vec<_>>()}]}})
            }).collect());
        }
        Ok(schema)
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

    pub fn response_schema(&self, operation: &Operation, schema: Value) -> Result<Value> {
        let mut schema = self.with_definitions(operation, schema)?;
        apply_schema_context(
            &mut schema,
            SchemaContext::Response,
            &self.definitions[&operation.version],
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
        if let Some(violations) = crate::schema::violations(&validator, input) {
            return Err(DatadogError::InvalidInput(format!(
                "Input for {} violates the API schema: {violations}",
                operation.id
            )));
        }
        Ok(())
    }
}

fn presence_schema(pointer: &str) -> Value {
    pointer
        .split('/')
        .skip(1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .fold(json!({}), |schema, key| {
            let key = key.replace("~1", "/").replace("~0", "~");
            json!({"type":"object","required":[key],"properties":BTreeMap::from([(key,schema)])})
        })
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

#[derive(Clone, Copy)]
enum SchemaContext {
    Request,
    Response,
}

fn apply_schema_context(schema: &mut Value, context: SchemaContext, definitions: &Value) {
    let Some(fields) = schema.as_object_mut() else {
        return;
    };
    let annotation = match context {
        SchemaContext::Request => "readOnly",
        SchemaContext::Response => "writeOnly",
    };
    let omitted = fields
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| {
            properties
                .iter()
                .filter(|(_, property)| has_annotation(property, annotation, definitions))
                .map(|(name, _)| name.clone())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    if let Some(required) = fields.get_mut("required").and_then(Value::as_array_mut) {
        required.retain(|name| name.as_str().is_none_or(|name| !omitted.contains(name)));
    }
    // Datadog ignores some undeclared request fields, so a misspelled filter
    // would silently widen a query. Composition keywords never accompany
    // declared request properties; tests/catalog_test.rs enforces that.
    if matches!(context, SchemaContext::Request)
        && fields.contains_key("properties")
        && !fields.contains_key("additionalProperties")
        && !fields.contains_key("patternProperties")
    {
        fields.insert("additionalProperties".into(), Value::Bool(false));
    }
    for keyword in ["properties", "patternProperties", "definitions", "$defs"] {
        if let Some(values) = fields.get_mut(keyword).and_then(Value::as_object_mut) {
            for value in values.values_mut() {
                apply_schema_context(value, context, definitions);
            }
        }
    }
    for keyword in [
        "items",
        "additionalProperties",
        "not",
        "contains",
        "propertyNames",
    ] {
        if let Some(value) = fields.get_mut(keyword) {
            apply_schema_context(value, context, definitions);
        }
    }
    for keyword in ["allOf", "oneOf", "anyOf"] {
        if let Some(values) = fields.get_mut(keyword).and_then(Value::as_array_mut) {
            for value in values {
                apply_schema_context(value, context, definitions);
            }
        }
    }
}

fn has_annotation<'a>(mut schema: &'a Value, annotation: &str, definitions: &'a Value) -> bool {
    let mut visited = BTreeSet::new();
    loop {
        if schema.get(annotation).and_then(Value::as_bool) == Some(true) {
            return true;
        }
        let Some(name) = schema
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|reference| reference.strip_prefix("#/definitions/"))
        else {
            return false;
        };
        if !visited.insert(name) {
            return false;
        }
        let Some(definition) = definitions.get(name) else {
            return false;
        };
        schema = definition;
    }
}

#[cfg(test)]
mod schema_context_tests {
    use super::*;

    #[test]
    fn directional_requirements_leave_instance_values_and_opposite_context_intact() {
        let original = json!({"type":"object","properties":{
            "id":{"$ref":"#/definitions/Id"},
            "secret":{"type":"string","writeOnly":true},
            "required":{"default":{"properties":{"id":{"readOnly":true}},"required":["id"]}}
        },"required":["id","secret","required"]});
        let definitions = json!({"Id":{"type":"string","readOnly":true}});
        let mut request = original.clone();
        apply_schema_context(&mut request, SchemaContext::Request, &definitions);
        assert_eq!(request["required"], json!(["secret", "required"]));
        let mut response = original.clone();
        apply_schema_context(&mut response, SchemaContext::Response, &definitions);
        assert_eq!(response["required"], json!(["id", "required"]));
        assert_eq!(request["properties"], original["properties"]);
        assert_eq!(response["properties"], original["properties"]);
    }

    #[test]
    fn requests_reject_undeclared_properties_unless_the_schema_allows_them() {
        let original = json!({"type":"object","properties":{
            "filter":{"type":"object","properties":{"query":{"type":"string"}}},
            "tags":{"type":"object","additionalProperties":{"type":"string"}},
            "open":{"type":"object","properties":{},"additionalProperties":true},
            "custom":{"type":"object","patternProperties":{"^x-":{}},"properties":{}}
        }});
        let mut request = original.clone();
        apply_schema_context(&mut request, SchemaContext::Request, &json!({}));
        assert_eq!(request["additionalProperties"], false);
        assert_eq!(
            request["properties"]["filter"]["additionalProperties"],
            false
        );
        for name in ["tags", "open", "custom"] {
            assert_eq!(request["properties"][name], original["properties"][name]);
        }
        let mut response = original.clone();
        apply_schema_context(&mut response, SchemaContext::Response, &json!({}));
        assert_eq!(response, original);
    }
}
