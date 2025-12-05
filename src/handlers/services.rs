use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{ParameterParser, ResponseFormatter};

pub struct ServicesHandler;

impl ResponseFormatter for ServicesHandler {}
impl ParameterParser for ServicesHandler {}

impl ServicesHandler {
    pub async fn list(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = ServicesHandler;

        let page_size = handler.extract_i32(params, "page_size", 100);
        let page = handler.extract_i32(params, "page", 0);
        let filter_env = handler.extract_string(params, "env");

        let response = client
            .get_service_catalog(Some(page_size), Some(page), filter_env.clone())
            .await?;

        let data: Vec<Value> = response
            .data
            .iter()
            .map(|service| {
                let mut s = json!({
                    "id": service.id,
                    "type": service.service_type,
                });

                if let Some(attrs) = &service.attributes {
                    s["schema_version"] = json!(attrs.schema_version);
                    s["dd_service"] = json!(attrs.dd_service);
                    s["dd_team"] = json!(attrs.dd_team);
                    s["application"] = json!(attrs.application);
                    s["tier"] = json!(attrs.tier);
                    s["lifecycle"] = json!(attrs.lifecycle);
                    s["type_of_service"] = json!(attrs.type_of_service);
                    s["languages"] = json!(attrs.languages);
                    s["tags"] = json!(attrs.tags);

                    if let Some(contacts) = &attrs.contacts {
                        s["contacts"] = json!(
                            contacts
                                .iter()
                                .map(|c| json!({
                                    "name": c.name,
                                    "email": c.email,
                                    "type": c.contact_type
                                }))
                                .collect::<Vec<_>>()
                        );
                    }

                    if let Some(links) = &attrs.links {
                        s["links"] = json!(
                            links
                                .iter()
                                .map(|l| json!({
                                    "name": l.name,
                                    "url": l.url,
                                    "type": l.link_type
                                }))
                                .collect::<Vec<_>>()
                        );
                    }

                    if let Some(repos) = &attrs.repos {
                        s["repos"] = json!(
                            repos
                                .iter()
                                .map(|r| json!({
                                    "name": r.name,
                                    "url": r.url,
                                    "provider": r.provider
                                }))
                                .collect::<Vec<_>>()
                        );
                    }

                    if let Some(docs) = &attrs.docs {
                        s["docs"] = json!(
                            docs.iter()
                                .map(|d| json!({
                                    "name": d.name,
                                    "url": d.url,
                                    "provider": d.provider
                                }))
                                .collect::<Vec<_>>()
                        );
                    }

                    if let Some(integrations) = &attrs.integrations {
                        let mut i = json!({});
                        if let Some(pagerduty) = &integrations.pagerduty {
                            i["pagerduty"] = pagerduty.clone();
                        }
                        if let Some(slack) = &integrations.slack {
                            i["slack"] = slack.clone();
                        }
                        for (key, value) in &integrations.others {
                            i[key] = value.clone();
                        }
                        s["integrations"] = i;
                    }

                    for (key, value) in &attrs.extra {
                        if let Some(obj) = s.as_object()
                            && !obj.contains_key(key)
                        {
                            s[key] = value.clone();
                        }
                    }
                }

                s
            })
            .collect();

        let pagination = handler.format_pagination(page as usize, page_size as usize, data.len());

        let meta = json!({
            "filter_env": filter_env,
            "warnings": response.meta.as_ref().and_then(|m| m.warnings.clone()).unwrap_or_default(),
            "next": response.links.as_ref().and_then(|l| l.next.clone())
        });

        Ok(handler.format_list(json!(data), Some(pagination), Some(meta)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_parser() {
        let handler = ServicesHandler;
        let params = json!({
            "env": "production",
            "page_size": 50,
            "page": 2,
        });

        assert_eq!(
            handler.extract_string(&params, "env"),
            Some("production".to_string())
        );
        assert_eq!(handler.extract_i32(&params, "page_size", 100), 50);
        assert_eq!(handler.extract_i32(&params, "page", 0), 2);
    }
}
