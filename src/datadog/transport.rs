use reqwest::{Client, Response, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::time::Duration;

use super::retry;
use crate::config::{Config, Credentials};
use crate::error::{DatadogError, Result};

pub struct DatadogClient {
    client: Client,
    base_url: String,
    max_retries: u32,
    total_timeout: Duration,
    tag_filter: Option<String>,
}

impl DatadogClient {
    pub fn new(config: &Config) -> Result<Self> {
        Self::with_base_url(config, format!("https://api.{}", config.site))
    }

    /// Points the client at an arbitrary base URL instead of the site's
    /// API host — for integration tests against a local mock server.
    pub fn with_base_url(config: &Config, base_url: String) -> Result<Self> {
        let client = Client::builder()
            .default_headers(auth_headers(&config.credentials)?)
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .timeout(Duration::from_secs(config.network.timeout_secs))
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .map_err(DatadogError::NetworkError)?;

        Ok(Self {
            client,
            base_url,
            max_retries: config.network.max_retries,
            total_timeout: Duration::from_secs(config.network.total_timeout_secs),
            tag_filter: config.defaults.tag_filter.clone(),
        })
    }

    pub fn get_tag_filter(&self) -> Option<&str> {
        self.tag_filter.as_deref()
    }

    pub(super) async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        query: Option<Vec<(&str, String)>>,
        body: Option<impl Serialize>,
    ) -> Result<T> {
        self.request_with_content_type(method, endpoint, query, body, "application/json")
            .await
    }

    pub(super) async fn request_with_content_type<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        query: Option<Vec<(&str, String)>>,
        body: Option<impl Serialize>,
        content_type: &str,
    ) -> Result<T> {
        tokio::time::timeout(
            self.total_timeout,
            self.send_with_retries(method, endpoint, query, body, content_type),
        )
        .await
        .map_err(|_| DatadogError::TimeoutError)?
    }

    async fn send_with_retries<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        query: Option<Vec<(&str, String)>>,
        body: Option<impl Serialize>,
        content_type: &str,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint);

        let mut attempt = 0;
        loop {
            let mut request = self
                .client
                .request(method.clone(), &url)
                .header("Content-Type", content_type);

            if let Some(ref params) = query {
                request = request.query(params);
            }

            if let Some(ref data) = body {
                request = request.json(data);
            }

            let result = match request.send().await {
                Ok(response) => self.handle_response(response).await,
                Err(e) if e.is_builder() => Err(DatadogError::InvalidInput(format!(
                    "Cannot build request: {}",
                    e.without_url()
                ))),
                Err(e) if e.is_timeout() => Err(DatadogError::TimeoutError),
                Err(e) => Err(DatadogError::NetworkError(e.without_url())),
            };

            match result {
                Ok(data) => return Ok(data),
                Err(error) => match retry::next_delay(&error, attempt, self.max_retries) {
                    Some(delay) => {
                        attempt += 1;
                        tracing::warn!(
                            attempt,
                            delay_ms = delay.as_millis() as u64,
                            %error,
                            "retrying request"
                        );
                        tokio::time::sleep(delay).await;
                    }
                    None => return Err(error),
                },
            }
        }
    }

    async fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            return response.json::<T>().await.map_err(|e| {
                if e.is_timeout() {
                    DatadogError::TimeoutError
                } else if e.is_decode() {
                    DatadogError::DecodeError(e.to_string())
                } else {
                    DatadogError::NetworkError(e.without_url())
                }
            });
        }

        if status == StatusCode::TOO_MANY_REQUESTS {
            let reset_secs = response
                .headers()
                .get("x-ratelimit-reset")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .or_else(|| retry::retry_after(response.headers()));
            return Err(DatadogError::RateLimitError { reset_secs });
        }

        let message = response.text().await.map_err(|e| {
            if e.is_timeout() {
                DatadogError::TimeoutError
            } else {
                DatadogError::NetworkError(e.without_url())
            }
        })?;

        match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(DatadogError::AuthError(message))
            }
            _ => Err(DatadogError::ApiError {
                status: status.as_u16(),
                message,
            }),
        }
    }
}

fn auth_headers(credentials: &Credentials) -> Result<reqwest::header::HeaderMap> {
    let mut headers = reqwest::header::HeaderMap::new();
    let mut insert = |name: &'static str, value: &str| -> Result<()> {
        let mut value = reqwest::header::HeaderValue::from_str(value).map_err(|_| {
            DatadogError::InvalidInput("Credentials contain invalid HTTP header characters".into())
        })?;
        value.set_sensitive(true);
        headers.insert(name, value);
        Ok(())
    };
    match credentials {
        Credentials::Keys { api_key, app_key } => {
            insert("dd-api-key", api_key)?;
            insert("dd-application-key", app_key)?;
        }
        Credentials::Token(token) => insert("authorization", &format!("Bearer {token}"))?,
    }
    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Credentials};
    use crate::config::{Defaults, Network};

    fn test_config(site: &str) -> Config {
        Config {
            credentials: Credentials::Keys {
                api_key: "test_api_key".to_string(),
                app_key: "test_app_key".to_string(),
            },
            site: site.to_string(),
            defaults: Defaults::default(),
            network: Network::default(),
        }
    }

    #[test]
    fn test_client_regional_urls() {
        let regions = vec![
            ("datadoghq.com", "https://api.datadoghq.com"),
            ("datadoghq.eu", "https://api.datadoghq.eu"),
            ("us3.datadoghq.com", "https://api.us3.datadoghq.com"),
            ("us5.datadoghq.com", "https://api.us5.datadoghq.com"),
        ];

        for (region, expected_url) in regions {
            let client = DatadogClient::new(&test_config(region)).unwrap();
            assert_eq!(client.base_url, expected_url);
        }
    }

    #[test]
    fn test_client_with_base_url_override() {
        let client = DatadogClient::with_base_url(
            &test_config("datadoghq.com"),
            "http://127.0.0.1:8080".to_string(),
        )
        .unwrap();

        assert_eq!(client.base_url, "http://127.0.0.1:8080");
    }

    #[test]
    fn test_tag_filter() {
        let mut config = test_config("datadoghq.com");
        config.defaults.tag_filter = Some("env:,service:".to_string());

        let client = DatadogClient::new(&config).unwrap();
        assert_eq!(client.get_tag_filter(), Some("env:,service:"));
    }

    #[test]
    fn test_network_settings_applied() {
        let mut config = test_config("datadoghq.com");
        config.network.max_retries = 5;

        let client = DatadogClient::new(&config).unwrap();
        assert_eq!(client.max_retries, 5);
    }
    #[test]
    fn authentication_headers_are_sensitive_and_invalid_values_are_rejected() {
        let headers = auth_headers(&Credentials::Token("secret".into())).unwrap();
        assert!(headers["authorization"].is_sensitive());
        assert!(!format!("{headers:?}").contains("secret"));
        assert!(auth_headers(&Credentials::Token("secret\nvalue".into())).is_err());
    }
}
