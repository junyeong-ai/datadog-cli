use crate::error::{DatadogError, Result};
use serde::Deserialize;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;
use std::{env, fs};

/// All Datadog regional sites (https://docs.datadoghq.com/getting_started/site/).
pub const VALID_SITES: &[&str] = &[
    "datadoghq.com",
    "us3.datadoghq.com",
    "us5.datadoghq.com",
    "datadoghq.eu",
    "ap1.datadoghq.com",
    "ap2.datadoghq.com",
    "uk1.datadoghq.com",
    "ddog-gov.com",
    "us2.ddog-gov.com",
];

/// Fully resolved, validated configuration. Credentials are guaranteed
/// present and the site is guaranteed valid once this exists.
#[derive(Debug, Clone)]
pub struct Config {
    pub credentials: Credentials,
    pub site: String,
    pub defaults: Defaults,
    pub network: Network,
}

/// Datadog authentication: classic API + application key pair, or a
/// personal access token (`ddpat_…`) sent as a Bearer token. A configured
/// token takes precedence over keys.
#[derive(Clone)]
pub enum Credentials {
    Keys { api_key: String, app_key: String },
    Token(String),
}

/// Never prints secret material, so a stray `{:?}` on a `Config` or
/// `Credentials` cannot leak keys into logs.
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Credentials::Keys { .. } => f.write_str("Keys { api_key: \"***\", app_key: \"***\" }"),
            Credentials::Token(_) => f.write_str("Token(\"***\")"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Defaults {
    pub format: String,
    pub tag_filter: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Network {
    pub timeout_secs: u64,
    pub max_retries: u32,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            format: "json".to_string(),
            tag_filter: None,
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_retries: 3,
        }
    }
}

pub const DEFAULT_SITE: &str = "datadoghq.com";

/// On-disk configuration schema. Every field is optional so that merging
/// across tiers (global file, project file, env, CLI) is presence-based:
/// a tier overrides only the fields it actually sets.
#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    api_key: Option<String>,
    app_key: Option<String>,
    token: Option<String>,
    site: Option<String>,
    #[serde(default)]
    defaults: DefaultsFile,
    #[serde(default)]
    network: NetworkFile,
}

#[derive(Debug, Default, Deserialize)]
struct DefaultsFile {
    format: Option<String>,
    tag_filter: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct NetworkFile {
    timeout_secs: Option<u64>,
    max_retries: Option<u32>,
}

impl ConfigFile {
    /// Priority: file < env < CLI args.
    fn merged(
        cli_api_key: Option<String>,
        cli_app_key: Option<String>,
        cli_token: Option<String>,
        cli_site: Option<String>,
    ) -> Result<Self> {
        let mut config = Self::load_from_file()?.unwrap_or_default();

        if let Ok(key) = env::var("DD_API_KEY") {
            config.api_key = Some(key);
        }
        if let Ok(key) = env::var("DD_APP_KEY") {
            config.app_key = Some(key);
        }
        if let Ok(token) = env::var("DD_TOKEN") {
            config.token = Some(token);
        }
        if let Ok(site) = env::var("DD_SITE") {
            config.site = Some(site);
        }
        if let Ok(filter) = env::var("DD_TAG_FILTER") {
            config.defaults.tag_filter = Some(filter);
        }

        if cli_api_key.is_some() {
            config.api_key = cli_api_key;
        }
        if cli_app_key.is_some() {
            config.app_key = cli_app_key;
        }
        if cli_token.is_some() {
            config.token = cli_token;
        }
        if cli_site.is_some() {
            config.site = cli_site;
        }

        Ok(config)
    }

    /// Reads the nearest config file. A missing file is fine (`None`);
    /// an unreadable or malformed file is an error, never silently ignored.
    fn load_from_file() -> Result<Option<Self>> {
        let path = Config::find_project_config()
            .or_else(Config::global_config_path)
            .ok_or_else(|| DatadogError::InvalidInput("Cannot determine config path".into()))?;

        if !path.exists() {
            return Ok(None);
        }

        #[cfg(unix)]
        Config::check_permissions(&path)?;

        let content = fs::read_to_string(&path)?;
        let config: ConfigFile = toml::from_str(&content).map_err(|e| {
            DatadogError::InvalidInput(format!("Invalid TOML in {}: {}", path.display(), e))
        })?;

        Ok(Some(config))
    }
}

impl Config {
    pub fn load(
        cli_api_key: Option<String>,
        cli_app_key: Option<String>,
        cli_token: Option<String>,
        cli_site: Option<String>,
    ) -> Result<Self> {
        let file = ConfigFile::merged(cli_api_key, cli_app_key, cli_token, cli_site)?;
        Self::resolve(file)
    }

    fn resolve(file: ConfigFile) -> Result<Self> {
        let credentials = match file.token.filter(|t| !t.is_empty()) {
            Some(token) => Credentials::Token(token),
            None => {
                let api_key = file.api_key.filter(|k| !k.is_empty());
                let app_key = file.app_key.filter(|k| !k.is_empty());
                match (api_key, app_key) {
                    (Some(api_key), Some(app_key)) => Credentials::Keys { api_key, app_key },
                    _ => {
                        return Err(DatadogError::AuthError(
                            "credentials required: set a personal access token (--token, \
                             DD_TOKEN env, or `token` in config) or both api_key and app_key \
                             (--api-key/--app-key, DD_API_KEY/DD_APP_KEY env, or config file)"
                                .into(),
                        ));
                    }
                }
            }
        };

        let site = file.site.unwrap_or_else(|| DEFAULT_SITE.to_string());
        if !VALID_SITES.contains(&site.as_str()) {
            return Err(DatadogError::InvalidInput(format!(
                "Invalid site: {}. Valid sites: {}",
                site,
                VALID_SITES.join(", ")
            )));
        }

        let base = Defaults::default();
        let defaults = Defaults {
            format: file.defaults.format.unwrap_or(base.format),
            tag_filter: file.defaults.tag_filter,
        };

        let base = Network::default();
        let network = Network {
            timeout_secs: file.network.timeout_secs.unwrap_or(base.timeout_secs),
            max_retries: file.network.max_retries.unwrap_or(base.max_retries),
        };

        Ok(Self {
            credentials,
            site,
            defaults,
            network,
        })
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
# Or authenticate with a personal access token instead of the key pair:
# token = "ddpat_..."
site = "datadoghq.com"

[defaults]
format = "json"
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

    /// Displays the merged configuration without requiring valid credentials,
    /// so it stays usable for debugging an incomplete setup.
    pub fn show() -> Result<String> {
        let file = ConfigFile::merged(None, None, None, None)?;
        let path = Self::global_config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let defaults = Defaults::default();
        let network = Network::default();

        Ok(format!(
            "Config: {}\n\
             API Key: {}\n\
             App Key: {}\n\
             Token: {}\n\
             Site: {}\n\n\
             [defaults]\n\
             format: {}\n\
             tag_filter: {}\n\n\
             [network]\n\
             timeout_secs: {}\n\
             max_retries: {}",
            path,
            file.api_key
                .as_deref()
                .map_or("(not set)".to_string(), mask_token),
            file.app_key
                .as_deref()
                .map_or("(not set)".to_string(), mask_token),
            file.token
                .as_deref()
                .map_or("(not set)".to_string(), mask_token),
            file.site.as_deref().unwrap_or(DEFAULT_SITE),
            file.defaults.format.unwrap_or(defaults.format),
            file.defaults.tag_filter.as_deref().unwrap_or("(none)"),
            file.network.timeout_secs.unwrap_or(network.timeout_secs),
            file.network.max_retries.unwrap_or(network.max_retries),
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
}

fn mask_token(token: &str) -> String {
    let chars: Vec<char> = token.chars().collect();
    if chars.len() > 8 {
        let head: String = chars[..4].iter().collect();
        let tail: String = chars[chars.len() - 4..].iter().collect();
        format!("{}...{}", head, tail)
    } else {
        "***".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_with_keys() -> ConfigFile {
        ConfigFile {
            api_key: Some("test123".to_string()),
            app_key: Some("app456".to_string()),
            ..ConfigFile::default()
        }
    }

    fn assert_keys(config: &Config) {
        match &config.credentials {
            Credentials::Keys { api_key, app_key } => {
                assert_eq!(api_key, "test123");
                assert_eq!(app_key, "app456");
            }
            other => panic!("expected key credentials, got {other:?}"),
        }
    }

    #[test]
    fn test_resolve_applies_defaults() {
        let config = Config::resolve(file_with_keys()).unwrap();
        assert_keys(&config);
        assert_eq!(config.site, "datadoghq.com");
        assert_eq!(config.defaults.format, "json");
        assert_eq!(config.network.timeout_secs, 30);
        assert_eq!(config.network.max_retries, 3);
    }

    #[test]
    fn test_resolve_missing_api_key() {
        let file = ConfigFile {
            app_key: Some("app456".to_string()),
            ..ConfigFile::default()
        };
        assert!(Config::resolve(file).is_err());
    }

    #[test]
    fn test_resolve_empty_api_key() {
        let file = ConfigFile {
            api_key: Some(String::new()),
            app_key: Some("app456".to_string()),
            ..ConfigFile::default()
        };
        assert!(Config::resolve(file).is_err());
    }

    #[test]
    fn test_resolve_token_only() {
        let file = ConfigFile {
            token: Some("ddpat_abc".to_string()),
            ..ConfigFile::default()
        };
        let config = Config::resolve(file).unwrap();
        assert!(matches!(config.credentials, Credentials::Token(t) if t == "ddpat_abc"));
    }

    #[test]
    fn test_resolve_token_takes_precedence_over_keys() {
        let file = ConfigFile {
            token: Some("ddpat_abc".to_string()),
            ..file_with_keys()
        };
        let config = Config::resolve(file).unwrap();
        assert!(matches!(config.credentials, Credentials::Token(_)));
    }

    #[test]
    fn test_resolve_empty_token_falls_back_to_keys() {
        let file = ConfigFile {
            token: Some(String::new()),
            ..file_with_keys()
        };
        let config = Config::resolve(file).unwrap();
        assert_keys(&config);
    }

    #[test]
    fn test_resolve_no_credentials() {
        assert!(Config::resolve(ConfigFile::default()).is_err());
    }

    #[test]
    fn test_resolve_invalid_site() {
        let file = ConfigFile {
            site: Some("datadoghq.evil.com".to_string()),
            ..file_with_keys()
        };
        assert!(Config::resolve(file).is_err());
    }

    #[test]
    fn test_resolve_all_valid_sites() {
        for site in VALID_SITES {
            let file = ConfigFile {
                site: Some(site.to_string()),
                ..file_with_keys()
            };
            assert_eq!(Config::resolve(file).unwrap().site, *site);
        }
    }

    #[test]
    fn test_resolve_partial_file_overrides() {
        let file = ConfigFile {
            defaults: DefaultsFile {
                format: Some("table".to_string()),
                ..DefaultsFile::default()
            },
            network: NetworkFile {
                timeout_secs: Some(60),
                ..NetworkFile::default()
            },
            ..file_with_keys()
        };

        let config = Config::resolve(file).unwrap();
        assert_eq!(config.defaults.format, "table");
        assert_eq!(config.network.timeout_secs, 60);
        assert_eq!(config.network.max_retries, 3);
    }

    #[test]
    fn test_config_file_parses_partial_toml() {
        let file: ConfigFile = toml::from_str(
            r#"
            api_key = "k"

            [network]
            max_retries = 5
            "#,
        )
        .unwrap();

        assert_eq!(file.api_key.as_deref(), Some("k"));
        assert_eq!(file.network.max_retries, Some(5));
        assert_eq!(file.network.timeout_secs, None);
        assert_eq!(file.defaults.format, None);
    }

    #[test]
    fn test_config_file_rejects_invalid_toml() {
        assert!(toml::from_str::<ConfigFile>("api_key = [broken").is_err());
    }

    #[test]
    fn test_mask_token() {
        assert_eq!(mask_token("abcdefghijklmnop"), "abcd...mnop");
        assert_eq!(mask_token("short"), "***");
    }

    #[test]
    fn test_mask_token_multibyte() {
        assert_eq!(mask_token("aaa\u{3042}aaaaaaaaaaaaa"), "aaaあ...aaaa");
        assert_eq!(mask_token("あいうえおかきくけこ"), "あいうえ...きくけこ");
        assert_eq!(mask_token("あいう"), "***");
    }

    #[test]
    fn test_credentials_debug_redacts_secrets() {
        let keys = Credentials::Keys {
            api_key: "secret-api".to_string(),
            app_key: "secret-app".to_string(),
        };
        let token = Credentials::Token("ddpat_secret".to_string());

        assert!(!format!("{:?}", keys).contains("secret"));
        assert!(!format!("{:?}", token).contains("secret"));
    }
}
