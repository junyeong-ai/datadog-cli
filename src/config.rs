use crate::error::{DatadogError, Result};
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};
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
    pub total_timeout_secs: u64,
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
            total_timeout_secs: 120,
        }
    }
}

pub const DEFAULT_SITE: &str = "datadoghq.com";

/// On-disk configuration schema. Every field is optional so that merging
/// across tiers (global file, project file, env, CLI) is presence-based:
/// a tier overrides only the fields it actually sets.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
struct DefaultsFile {
    format: Option<String>,
    tag_filter: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct NetworkFile {
    timeout_secs: Option<u64>,
    max_retries: Option<u32>,
    total_timeout_secs: Option<u64>,
}

impl ConfigFile {
    /// Priority: file < env < CLI args.
    fn merged(
        cli_api_key: Option<String>,
        cli_app_key: Option<String>,
        cli_token: Option<String>,
        cli_site: Option<String>,
    ) -> Result<Self> {
        let mut config = Self::load_paths(
            Config::global_config_path().as_deref(),
            Config::find_project_config()?.as_deref(),
        )?;
        config.overlay(Self {
            api_key: env_value("DD_API_KEY")?,
            app_key: env_value("DD_APP_KEY")?,
            token: env_value("DD_TOKEN")?,
            site: env_value("DD_SITE")?,
            defaults: DefaultsFile {
                tag_filter: env_value("DD_TAG_FILTER")?,
                ..DefaultsFile::default()
            },
            ..Self::default()
        });
        config.overlay(Self {
            api_key: cli_api_key,
            app_key: cli_app_key,
            token: cli_token,
            site: cli_site,
            ..Self::default()
        });

        Ok(config)
    }

    fn overlay(&mut self, higher: Self) {
        if higher.token.is_none() && (higher.api_key.is_some() || higher.app_key.is_some()) {
            self.token = None;
        }
        self.api_key = higher.api_key.or(self.api_key.take());
        self.app_key = higher.app_key.or(self.app_key.take());
        self.token = higher.token.or(self.token.take());
        self.site = higher.site.or(self.site.take());
        self.defaults.format = higher.defaults.format.or(self.defaults.format.take());
        self.defaults.tag_filter = higher
            .defaults
            .tag_filter
            .or(self.defaults.tag_filter.take());
        self.network.timeout_secs = higher.network.timeout_secs.or(self.network.timeout_secs);
        self.network.max_retries = higher.network.max_retries.or(self.network.max_retries);
        self.network.total_timeout_secs = higher
            .network
            .total_timeout_secs
            .or(self.network.total_timeout_secs);
    }

    fn load_paths(global: Option<&Path>, project: Option<&Path>) -> Result<Self> {
        let mut config = Self::default();
        for path in [global, project].into_iter().flatten() {
            let content = match fs::read_to_string(path) {
                Ok(content) => content,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            #[cfg(unix)]
            Config::check_permissions(path)?;
            let layer = toml::from_str(&content).map_err(|error: toml::de::Error| {
                let prefix = &content.as_bytes()[..error.span().map_or(0, |span| span.start)];
                let line = prefix.iter().filter(|&&byte| byte == b'\n').count() + 1;
                DatadogError::InvalidInput(format!(
                    "Invalid config in {} at line {line}: check TOML syntax and field names",
                    path.display()
                ))
            })?;
            config.overlay(layer);
        }
        Ok(config)
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
            total_timeout_secs: file
                .network
                .total_timeout_secs
                .unwrap_or(base.total_timeout_secs),
        };

        if crate::output::Format::from_str(&defaults.format).is_err() {
            return Err(DatadogError::InvalidInput(
                "defaults.format must be json, jsonl (or jsonlines), or table".into(),
            ));
        }
        if network.timeout_secs == 0 || network.total_timeout_secs == 0 {
            return Err(DatadogError::InvalidInput(
                "network timeouts must be greater than zero".into(),
            ));
        }

        Ok(Self {
            credentials,
            site,
            defaults,
            network,
        })
    }

    fn find_project_config() -> Result<Option<PathBuf>> {
        let mut current = env::current_dir()?;
        loop {
            let candidate = current.join(".datadog.toml");
            match candidate.try_exists() {
                Ok(true) => return Ok(Some(candidate)),
                Ok(false) => {}
                Err(error) => return Err(error.into()),
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => return Ok(None),
            }
        }
    }

    pub fn active_config_path() -> Result<PathBuf> {
        Self::find_project_config()?
            .or_else(Self::global_config_path)
            .ok_or_else(|| DatadogError::InvalidInput("Cannot determine config path".into()))
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

        Self::init_at(path)
    }

    fn init_at(path: PathBuf) -> Result<PathBuf> {
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
total_timeout_secs = 120
"#;

        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(&path)?.write_all(template.as_bytes())?;

        Ok(path)
    }

    /// Displays the merged configuration without requiring valid credentials,
    /// so it stays usable for debugging an incomplete setup.
    pub fn show() -> Result<String> {
        Self::show_with_overrides(None, None, None, None)
    }

    pub fn show_with_overrides(
        api_key: Option<String>,
        app_key: Option<String>,
        token: Option<String>,
        site: Option<String>,
    ) -> Result<String> {
        let file = ConfigFile::merged(api_key, app_key, token, site)?;
        let path = Self::active_config_path()?.display().to_string();

        let defaults = Defaults::default();
        let network = Network::default();

        Ok(format!(
            "Config: {}\n\
             Global config: {}\n\
             API Key: {}\n\
             App Key: {}\n\
             Token: {}\n\
             Site: {}\n\n\
             [defaults]\n\
             format: {}\n\
             tag_filter: {}\n\n\
             [network]\n\
             timeout_secs: {}\n\
             max_retries: {}\n\
             total_timeout_secs: {}",
            path,
            Self::global_config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "(unavailable)".into()),
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
            file.network
                .total_timeout_secs
                .unwrap_or(network.total_timeout_secs),
        ))
    }

    pub fn edit() -> Result<()> {
        let path = Self::active_config_path()?;

        if !path.exists() {
            return Err(DatadogError::InvalidInput(format!(
                "Config not found: {}\nRun: datadog-cli config init",
                path.display()
            )));
        }

        let editor = env_value("EDITOR")?.unwrap_or_else(|| "vim".to_string());
        let status = editor_command(&editor, &path)?
            .status()
            .map_err(|e| DatadogError::InvalidInput(format!("Failed to launch editor: {}", e)))?;

        let mut interrupted = status.code() == Some(130);
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            interrupted |= status.signal() == Some(2);
        }
        #[cfg(windows)]
        {
            interrupted |= status.code() == Some(0xC000013Au32 as i32);
        }
        if interrupted {
            return Err(DatadogError::Interrupted);
        }
        if !status.success() {
            return Err(DatadogError::InvalidInput(
                "Editor exited with error".into(),
            ));
        }

        Ok(())
    }
}

fn editor_command(editor: &str, path: &Path) -> Result<std::process::Command> {
    if editor.contains('\0') {
        return Err(DatadogError::InvalidInput(
            "EDITOR must not contain NUL characters".into(),
        ));
    }
    #[cfg(not(windows))]
    let words = shlex::split(editor).ok_or_else(|| {
        DatadogError::InvalidInput("EDITOR contains invalid quoting or escapes".into())
    })?;
    #[cfg(windows)]
    let words = windows_args::Args::parse_cmd(editor.trim_start()).collect::<Vec<_>>();

    let (program, args) = words
        .split_first()
        .filter(|(program, _)| !program.is_empty())
        .ok_or_else(|| DatadogError::InvalidInput("EDITOR must specify an executable".into()))?;
    let mut command = std::process::Command::new(program);
    command.args(args).arg(path);
    Ok(command)
}

fn env_value(name: &str) -> Result<Option<String>> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(DatadogError::InvalidInput(format!(
            "{name} must contain valid Unicode"
        ))),
    }
}

fn mask_token(token: &str) -> String {
    if token.is_empty() { "(empty)" } else { "***" }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_commands_reject_empty_executables_and_nul() {
        for input in ["", "  ", "\"\"", "editor\0--wait"] {
            assert!(matches!(
                editor_command(input, Path::new("config.toml")),
                Err(DatadogError::InvalidInput(_))
            ));
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn editor_commands_reject_invalid_posix_quoting() {
        for input in ["code --wait '", r"code --wait \"] {
            assert!(editor_command(input, Path::new("config.toml")).is_err());
        }
    }

    #[cfg(windows)]
    #[test]
    fn editor_commands_preserve_windows_paths() {
        for input in [
            r"C:\Windows\notepad.exe",
            r#""C:\Program Files\Editor\editor.exe" --wait """#,
        ] {
            let command = editor_command(input, Path::new("config.toml")).unwrap();
            let program = command.get_program().to_str().unwrap();
            assert!(program.contains('\\'));
            assert!(program.ends_with(".exe"));
        }
    }

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
        assert_eq!(mask_token("abcdefghijklmnop"), "***");
        assert_eq!(mask_token("short"), "***");
    }

    #[test]
    fn test_mask_token_multibyte() {
        assert_eq!(mask_token("aaa\u{3042}aaaaaaaaaaaaa"), "***");
        assert_eq!(mask_token("あいうえおかきくけこ"), "***");
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
    #[test]
    fn config_layers_merge_individual_fields_and_auth_methods() {
        let mut lower = ConfigFile {
            token: Some("lower-token".into()),
            defaults: DefaultsFile {
                format: Some("jsonl".into()),
                tag_filter: Some("env:".into()),
            },
            ..file_with_keys()
        };
        lower.overlay(ConfigFile {
            api_key: Some("project-api".into()),
            network: NetworkFile {
                timeout_secs: Some(15),
                ..NetworkFile::default()
            },
            ..ConfigFile::default()
        });
        let config = Config::resolve(lower).unwrap();
        assert!(
            matches!(config.credentials, Credentials::Keys { api_key, app_key } if api_key == "project-api" && app_key == "app456")
        );
        assert_eq!(config.defaults.format, "jsonl");
        assert_eq!(config.defaults.tag_filter.as_deref(), Some("env:"));
        assert_eq!(config.network.timeout_secs, 15);
    }

    #[test]
    fn higher_token_wins_and_empty_token_explicitly_clears_it() {
        let mut config = file_with_keys();
        config.overlay(ConfigFile {
            token: Some("token".into()),
            ..ConfigFile::default()
        });
        assert_eq!(config.token.as_deref(), Some("token"));
        config.overlay(ConfigFile {
            token: Some(String::new()),
            ..ConfigFile::default()
        });
        assert_keys(&Config::resolve(config).unwrap());
    }

    #[test]
    fn config_files_merge_and_malformed_layers_fail_without_exposing_secrets() {
        let directory = tempfile::tempdir().unwrap();
        let global = directory.path().join("global.toml");
        let project = directory.path().join("project.toml");
        fs::write(
            &global,
            "api_key = 'global'\napp_key = 'app'\n[defaults]\nformat = 'jsonl'\n",
        )
        .unwrap();
        fs::write(&project, "site = 'datadoghq.eu'\n").unwrap();
        let merged =
            Config::resolve(ConfigFile::load_paths(Some(&global), Some(&project)).unwrap())
                .unwrap();
        assert_eq!(merged.site, "datadoghq.eu");
        assert_eq!(merged.defaults.format, "jsonl");
        fs::write(&global, "api_key = 'secret-value' broken").unwrap();
        let error = ConfigFile::load_paths(Some(&global), Some(&project))
            .unwrap_err()
            .to_string();
        assert!(!error.contains("secret-value"));
        assert!(error.contains("global.toml"));
    }

    #[test]
    fn config_rejects_unknown_fields_invalid_formats_and_zero_timeouts() {
        assert!(toml::from_str::<ConfigFile>("[network]\ntimeout_sec = 30").is_err());
        for source in [
            "[defaults]\nformat = 'yaml'",
            "[network]\ntimeout_secs = 0",
            "[network]\ntotal_timeout_secs = 0",
        ] {
            let mut file = file_with_keys();
            file.overlay(toml::from_str(source).unwrap());
            assert!(Config::resolve(file).is_err());
        }
    }
    #[test]
    fn initialization_is_private_and_never_overwrites_an_existing_config() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        Config::init_at(path.clone()).unwrap();
        let original = fs::read_to_string(&path).unwrap();
        assert!(Config::init_at(path.clone()).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    #[test]
    fn existing_config_format_aliases_remain_valid() {
        for format in ["jsonlines", "JSONL", "JSON", "Table"] {
            let mut file = file_with_keys();
            file.defaults.format = Some(format.into());
            assert_eq!(Config::resolve(file).unwrap().defaults.format, format);
        }
    }
}
