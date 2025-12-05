use crate::error::{DatadogError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_key: Option<String>,

    #[serde(default = "Defaults::default_site")]
    pub site: String,

    #[serde(default)]
    pub defaults: Defaults,

    #[serde(default)]
    pub network: Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default = "Defaults::default_format")]
    pub format: String,

    #[serde(default = "Defaults::default_time_range")]
    pub time_range: String,

    #[serde(default = "Defaults::default_limit")]
    pub limit: i32,

    #[serde(default = "Defaults::default_page_size")]
    pub page_size: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    #[serde(default = "Network::default_timeout_secs")]
    pub timeout_secs: u64,

    #[serde(default = "Network::default_max_retries")]
    pub max_retries: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: None,
            app_key: None,
            site: Defaults::default_site(),
            defaults: Defaults::default(),
            network: Network::default(),
        }
    }
}

impl Defaults {
    pub fn default_site() -> String {
        "datadoghq.com".to_string()
    }

    pub fn default_format() -> String {
        "json".to_string()
    }

    pub fn default_time_range() -> String {
        "1 hour ago".to_string()
    }

    pub fn default_limit() -> i32 {
        10
    }

    pub fn default_page_size() -> i32 {
        100
    }
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            format: Self::default_format(),
            time_range: Self::default_time_range(),
            limit: Self::default_limit(),
            page_size: Self::default_page_size(),
            tag_filter: None,
        }
    }
}

impl Network {
    pub fn default_timeout_secs() -> u64 {
        30
    }

    pub fn default_max_retries() -> u32 {
        3
    }
}

impl Default for Network {
    fn default() -> Self {
        Self {
            timeout_secs: Self::default_timeout_secs(),
            max_retries: Self::default_max_retries(),
        }
    }
}

impl Config {
    pub fn load(
        cli_api_key: Option<String>,
        cli_app_key: Option<String>,
        cli_site: Option<String>,
    ) -> Result<Self> {
        let mut config = Self::default();

        if let Ok(file_config) = Self::load_from_file() {
            config = config.merge(file_config);
        }

        if let Ok(key) = env::var("DD_API_KEY") {
            config.api_key = Some(key);
        }
        if let Ok(key) = env::var("DD_APP_KEY") {
            config.app_key = Some(key);
        }
        if let Ok(site) = env::var("DD_SITE") {
            config.site = site;
        }
        if let Ok(filter) = env::var("DD_TAG_FILTER") {
            config.defaults.tag_filter = Some(filter);
        }

        if let Some(key) = cli_api_key {
            config.api_key = Some(key);
        }
        if let Some(key) = cli_app_key {
            config.app_key = Some(key);
        }
        if let Some(site) = cli_site {
            config.site = site;
        }

        config.validate()?;
        Ok(config)
    }

    pub fn load_defaults_only() -> Self {
        Self::load_from_file().unwrap_or_default()
    }

    fn load_from_file() -> Result<Self> {
        let path = Self::find_project_config()
            .or_else(Self::global_config_path)
            .ok_or_else(|| DatadogError::InvalidInput("Cannot determine config path".into()))?;

        if !path.exists() {
            return Err(DatadogError::InvalidInput(format!(
                "Config not found: {}\nRun: datadog-cli config init",
                path.display()
            )));
        }

        #[cfg(unix)]
        Self::check_permissions(&path)?;

        let content = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| DatadogError::InvalidInput(format!("Invalid TOML: {}", e)))?;

        Ok(config)
    }

    fn find_project_config() -> Option<PathBuf> {
        let mut current = env::current_dir().ok()?;
        loop {
            let candidate = current.join(".datadog.toml");
            if candidate.exists() {
                return Some(candidate);
            }
            current = current.parent()?.to_path_buf();
        }
    }

    pub fn global_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".config/datadog-cli/config.toml"))
    }

    #[cfg(unix)]
    fn check_permissions(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(path)?;
        let mode = metadata.permissions().mode();

        if mode & 0o077 != 0 {
            eprintln!(
                "Warning: Config has insecure permissions: {:o}. Run: chmod 600 {}",
                mode,
                path.display()
            );
        }

        Ok(())
    }

    fn merge(mut self, other: Self) -> Self {
        if other.api_key.is_some() {
            self.api_key = other.api_key;
        }
        if other.app_key.is_some() {
            self.app_key = other.app_key;
        }
        if !other.site.is_empty() {
            self.site = other.site;
        }
        self.defaults = self.defaults.merge(other.defaults);
        self.network = other.network;
        self
    }

    fn validate(&self) -> Result<()> {
        if self.api_key.is_none() || self.api_key.as_ref().is_some_and(|k| k.is_empty()) {
            return Err(DatadogError::AuthError(
                "api_key required. Use --api-key, DD_API_KEY env, or config file".into(),
            ));
        }

        if self.app_key.is_none() || self.app_key.as_ref().is_some_and(|k| k.is_empty()) {
            return Err(DatadogError::AuthError(
                "app_key required. Use --app-key, DD_APP_KEY env, or config file".into(),
            ));
        }

        if !self.site.contains("datadoghq.") && !self.site.contains("ddog-gov.") {
            return Err(DatadogError::InvalidInput(format!(
                "Invalid site: {}",
                self.site
            )));
        }

        Ok(())
    }

    pub fn init() -> Result<PathBuf> {
        let path = Self::global_config_path()
            .ok_or_else(|| DatadogError::InvalidInput("Cannot determine config path".into()))?;

        if path.exists() {
            return Err(DatadogError::InvalidInput(format!(
                "Config already exists: {}",
                path.display()
            )));
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let template = r#"api_key = "your-api-key-here"
app_key = "your-app-key-here"
site = "datadoghq.com"

[defaults]
format = "json"
time_range = "1 hour ago"
limit = 10
page_size = 100
# tag_filter = "env:,service:"

[network]
timeout_secs = 30
max_retries = 3
"#;

        fs::write(&path, template)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&path, perms)?;
        }

        Ok(path)
    }

    pub fn show() -> Result<String> {
        let config = Self::load(None, None, None)?;
        let path = Self::global_config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let api_key = config.api_key.as_ref().unwrap();
        let app_key = config.app_key.as_ref().unwrap();

        Ok(format!(
            "Config: {}\n\
             API Key: {}\n\
             App Key: {}\n\
             Site: {}\n\n\
             [defaults]\n\
             format: {}\n\
             time_range: {}\n\
             limit: {}\n\
             page_size: {}\n\
             tag_filter: {}\n\n\
             [network]\n\
             timeout_secs: {}\n\
             max_retries: {}",
            path,
            mask_token(api_key),
            mask_token(app_key),
            config.site,
            config.defaults.format,
            config.defaults.time_range,
            config.defaults.limit,
            config.defaults.page_size,
            config.defaults.tag_filter.as_deref().unwrap_or("(none)"),
            config.network.timeout_secs,
            config.network.max_retries,
        ))
    }

    pub fn edit() -> Result<()> {
        let path = Self::global_config_path()
            .ok_or_else(|| DatadogError::InvalidInput("Cannot determine config path".into()))?;

        if !path.exists() {
            return Err(DatadogError::InvalidInput(format!(
                "Config not found: {}\nRun: datadog-cli config init",
                path.display()
            )));
        }

        let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

        let status = std::process::Command::new(&editor)
            .arg(&path)
            .status()
            .map_err(|e| DatadogError::InvalidInput(format!("Failed to launch editor: {}", e)))?;

        if !status.success() {
            return Err(DatadogError::InvalidInput(
                "Editor exited with error".into(),
            ));
        }

        Ok(())
    }

    pub fn api_key(&self) -> &str {
        self.api_key.as_ref().unwrap()
    }

    pub fn app_key(&self) -> &str {
        self.app_key.as_ref().unwrap()
    }
}

impl Defaults {
    fn merge(mut self, other: Self) -> Self {
        if other.format != Self::default_format() {
            self.format = other.format;
        }
        if other.time_range != Self::default_time_range() {
            self.time_range = other.time_range;
        }
        if other.limit != Self::default_limit() {
            self.limit = other.limit;
        }
        if other.page_size != Self::default_page_size() {
            self.page_size = other.page_size;
        }
        if other.tag_filter.is_some() {
            self.tag_filter = other.tag_filter;
        }
        self
    }
}

fn mask_token(token: &str) -> String {
    if token.len() > 8 {
        format!("{}...{}", &token[..4], &token[token.len() - 4..])
    } else {
        "***".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.site, "datadoghq.com");
        assert_eq!(config.defaults.format, "json");
        assert_eq!(config.defaults.time_range, "1 hour ago");
        assert_eq!(config.defaults.limit, 10);
        assert_eq!(config.defaults.page_size, 100);
        assert_eq!(config.network.timeout_secs, 30);
        assert_eq!(config.network.max_retries, 3);
    }

    #[test]
    fn test_validate_valid() {
        let config = Config {
            api_key: Some("test123".to_string()),
            app_key: Some("app456".to_string()),
            site: "datadoghq.com".to_string(),
            defaults: Defaults::default(),
            network: Network::default(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_missing_api_key() {
        let config = Config {
            api_key: None,
            app_key: Some("app456".to_string()),
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_site() {
        let config = Config {
            api_key: Some("test123".to_string()),
            app_key: Some("app456".to_string()),
            site: "invalid.com".to_string(),
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_mask_token() {
        assert_eq!(mask_token("abcdefghijklmnop"), "abcd...mnop");
        assert_eq!(mask_token("short"), "***");
    }

    #[test]
    fn test_defaults_merge() {
        let base = Defaults::default();
        let other = Defaults {
            format: "jsonl".to_string(),
            limit: 50,
            ..Defaults::default()
        };

        let merged = base.merge(other);
        assert_eq!(merged.format, "jsonl");
        assert_eq!(merged.limit, 50);
        assert_eq!(merged.page_size, 100);
    }

    #[test]
    fn test_network_defaults() {
        let network = Network::default();
        assert_eq!(network.timeout_secs, 30);
        assert_eq!(network.max_retries, 3);
    }
}
