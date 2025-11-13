use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::error::{DatadogError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub app_key: String,

    #[serde(default = "default_site")]
    pub site: String,
}

fn default_site() -> String {
    "datadoghq.com".to_string()
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path()
            .ok_or_else(|| DatadogError::InvalidInput("Cannot determine config path".into()))?;

        if !path.exists() {
            return Err(DatadogError::InvalidInput(format!(
                "Config file not found: {}\nRun: datadog config init",
                path.display()
            )));
        }

        #[cfg(unix)]
        Self::check_permissions(&path)?;

        let content = fs::read_to_string(&path)
            .map_err(|e| DatadogError::IoError(e))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| DatadogError::InvalidInput(format!("Invalid TOML: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    pub fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".config/datadog-cli/config.toml"))
    }

    #[cfg(unix)]
    fn check_permissions(path: &std::path::Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(path)?;
        let mode = metadata.permissions().mode();

        if mode & 0o077 != 0 {
            log::warn!(
                "Config file has insecure permissions: {:o}. Run: chmod 600 {}",
                mode,
                path.display()
            );
        }

        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(DatadogError::AuthError("api_key is empty".into()));
        }

        if self.app_key.is_empty() {
            return Err(DatadogError::AuthError("app_key is empty".into()));
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
        let path = Self::config_path()
            .ok_or_else(|| DatadogError::InvalidInput("Cannot determine config path".into()))?;

        if path.exists() {
            return Err(DatadogError::InvalidInput(format!(
                "Config file already exists: {}",
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
        let config = Self::load()?;
        let path = Self::config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(format!(
            "Config: {}\n\
             api_key: {}...{}\n\
             app_key: {}...{}\n\
             site: {}",
            path,
            &config.api_key[..8.min(config.api_key.len())],
            &config.api_key[config.api_key.len().saturating_sub(4)..],
            &config.app_key[..8.min(config.app_key.len())],
            &config.app_key[config.app_key.len().saturating_sub(4)..],
            config.site
        ))
    }

    pub fn edit() -> Result<()> {
        let path = Self::config_path()
            .ok_or_else(|| DatadogError::InvalidInput("Cannot determine config path".into()))?;

        if !path.exists() {
            return Err(DatadogError::InvalidInput(format!(
                "Config file not found: {}\nRun: datadog config init",
                path.display()
            )));
        }

        let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
            if cfg!(target_os = "macos") {
                "vim".to_string()
            } else if cfg!(target_os = "windows") {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        });

        let status = std::process::Command::new(&editor)
            .arg(&path)
            .status()
            .map_err(|e| DatadogError::InvalidInput(format!("Failed to launch editor: {}", e)))?;

        if !status.success() {
            return Err(DatadogError::InvalidInput("Editor exited with error".into()));
        }

        Ok(())
    }

    pub fn base_url(&self) -> String {
        format!("https://api.{}", self.site)
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
            api_key: "test123".to_string(),
            app_key: "app456".to_string(),
            site: "datadoghq.com".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_api_key() {
        let config = Config {
            api_key: String::new(),
            app_key: "app456".to_string(),
            site: "datadoghq.com".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_site() {
        let config = Config {
            api_key: "test123".to_string(),
            app_key: "app456".to_string(),
            site: "invalid.com".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_base_url() {
        let config = Config {
            api_key: "test".to_string(),
            app_key: "test".to_string(),
            site: "datadoghq.eu".to_string(),
        };
        assert_eq!(config.base_url(), "https://api.datadoghq.eu");
    }
}
