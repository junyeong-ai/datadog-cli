use crate::error::{DatadogError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_key: Option<String>,

    #[serde(default = "default_site")]
    pub site: String,
}

fn default_site() -> String {
    "datadoghq.com".to_string()
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

    fn load_from_file() -> Result<Self> {
        let path = Self::find_project_config()
            .or_else(Self::global_config_path)
            .ok_or_else(|| DatadogError::InvalidInput("Cannot determine config path".into()))?;

        if !path.exists() {
            return Err(DatadogError::InvalidInput(format!(
                "Config not found: {}\nRun: datadog config init",
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
            tracing::warn!(
                "Config has insecure permissions: {:o}. Run: chmod 600 {}",
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
        self
    }

    fn validate(&self) -> Result<()> {
        if self.api_key.is_none() || self.api_key.as_ref().unwrap().is_empty() {
            return Err(DatadogError::AuthError(
                "api_key required. Use --api-key, DD_API_KEY env, or config file".into(),
            ));
        }

        if self.app_key.is_none() || self.app_key.as_ref().unwrap().is_empty() {
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
            "Config: {}\nAPI Key: {}\nApp Key: {}\nSite: {}",
            path,
            mask_token(api_key),
            mask_token(app_key),
            config.site
        ))
    }

    pub fn edit() -> Result<()> {
        let path = Self::global_config_path()
            .ok_or_else(|| DatadogError::InvalidInput("Cannot determine config path".into()))?;

        if !path.exists() {
            return Err(DatadogError::InvalidInput(format!(
                "Config not found: {}\nRun: datadog config init",
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

    pub fn base_url(&self) -> String {
        format!("https://api.{}", self.site)
    }

    pub fn api_key(&self) -> &str {
        self.api_key.as_ref().unwrap()
    }

    pub fn app_key(&self) -> &str {
        self.app_key.as_ref().unwrap()
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
    fn test_default_site() {
        assert_eq!(default_site(), "datadoghq.com");
    }

    #[test]
    fn test_validate_valid() {
        let config = Config {
            api_key: Some("test123".to_string()),
            app_key: Some("app456".to_string()),
            site: "datadoghq.com".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_missing_api_key() {
        let config = Config {
            api_key: None,
            app_key: Some("app456".to_string()),
            site: "datadoghq.com".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_site() {
        let config = Config {
            api_key: Some("test123".to_string()),
            app_key: Some("app456".to_string()),
            site: "invalid.com".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_base_url() {
        let config = Config {
            api_key: Some("test".to_string()),
            app_key: Some("test".to_string()),
            site: "datadoghq.eu".to_string(),
        };
        assert_eq!(config.base_url(), "https://api.datadoghq.eu");
    }

    #[test]
    fn test_mask_token() {
        assert_eq!(mask_token("abcdefghijklmnop"), "abcd...mnop");
        assert_eq!(mask_token("short"), "***");
    }

    #[test]
    fn test_merge() {
        let base = Config {
            api_key: Some("base_key".to_string()),
            app_key: Some("base_app".to_string()),
            site: "datadoghq.com".to_string(),
        };

        let override_config = Config {
            api_key: Some("override_key".to_string()),
            app_key: None,
            site: default_site(),
        };

        let merged = base.merge(override_config);
        assert_eq!(merged.api_key, Some("override_key".to_string()));
        assert_eq!(merged.app_key, Some("base_app".to_string()));
    }
}
