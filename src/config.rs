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
/// PAT, SAT, or OAuth access token sent as a Bearer token. A configured
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum AuthMethod {
    ApiKeys,
    Token,
    Oauth,
}

impl AuthMethod {
    fn as_str(self) -> &'static str {
        match self {
            Self::ApiKeys => "api-keys",
            Self::Token => "token",
            Self::Oauth => "oauth",
        }
    }
}

#[derive(Default)]
pub struct ConfigOptions {
    pub api_key: Option<String>,
    pub app_key: Option<String>,
    pub token: Option<String>,
    pub site: Option<String>,
    pub auth_method: Option<AuthMethod>,
    pub pup_org: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthFile {
    method: AuthMethod,
    pup_org: Option<String>,
}

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
    auth: Option<AuthFile>,
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
    fn retain_credentials(&mut self, method: Option<AuthMethod>) {
        match method {
            Some(AuthMethod::ApiKeys) => self.token = None,
            Some(AuthMethod::Token) => {
                self.api_key = None;
                self.app_key = None;
            }
            Some(AuthMethod::Oauth) => {
                self.api_key = None;
                self.app_key = None;
                self.token = None;
            }
            None => {}
        }
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

    fn read(path: Option<&Path>) -> Result<Self> {
        if let Some(path) = path {
            let content = match fs::read_to_string(path) {
                Ok(content) => content,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(Self::default());
                }
                Err(error) => return Err(error.into()),
            };
            #[cfg(unix)]
            Config::check_permissions(path)?;
            return toml::from_str(&content).map_err(|error: toml::de::Error| {
                let prefix = &content.as_bytes()[..error.span().map_or(0, |span| span.start)];
                let line = prefix.iter().filter(|&&byte| byte == b'\n').count() + 1;
                DatadogError::InvalidInput(format!(
                    "Invalid config in {} at line {line}: check TOML syntax and field names",
                    path.display()
                ))
            });
        }
        Ok(Self::default())
    }
}

#[derive(Debug)]
struct ConfigFiles {
    global: ConfigFile,
    project: ConfigFile,
}

impl ConfigFiles {
    fn load_paths(global: Option<&Path>, project: Option<&Path>) -> Result<Self> {
        let files = Self {
            global: ConfigFile::read(global)?,
            project: ConfigFile::read(project)?,
        };
        if files.project.auth.is_some() {
            return Err(DatadogError::InvalidInput(
                "[auth] is only supported in the global config.toml, not project .datadog.toml"
                    .into(),
            ));
        }
        Ok(files)
    }

    fn merged(mut self, method: Option<AuthMethod>) -> ConfigFile {
        self.global.retain_credentials(method);
        self.project.retain_credentials(method);
        self.global.overlay(self.project);
        self.global
    }
}

struct ConfigPlan {
    file: ConfigFile,
    method: Option<AuthMethod>,
    source: &'static str,
    pup_org: Option<String>,
}

impl ConfigPlan {
    fn load(options: ConfigOptions) -> Result<Self> {
        let files = ConfigFiles::load_paths(
            Config::global_config_path().as_deref(),
            Config::find_project_config()?.as_deref(),
        )?;
        let (method, _) = Self::select_method(&files, &options);
        let environment = ConfigFile {
            api_key: if matches!(method, None | Some(AuthMethod::ApiKeys)) {
                env_value("DD_API_KEY")?
            } else {
                None
            },
            app_key: if matches!(method, None | Some(AuthMethod::ApiKeys)) {
                env_value("DD_APP_KEY")?
            } else {
                None
            },
            token: if matches!(method, None | Some(AuthMethod::Token)) {
                env_value("DD_TOKEN")?
            } else {
                None
            },
            site: env_value("DD_SITE")?,
            defaults: DefaultsFile {
                tag_filter: env_value("DD_TAG_FILTER")?,
                ..DefaultsFile::default()
            },
            ..ConfigFile::default()
        };
        Self::resolve(files, environment, options)
    }

    fn select_method(
        files: &ConfigFiles,
        options: &ConfigOptions,
    ) -> (Option<AuthMethod>, &'static str) {
        let cli_method = options.auth_method.or_else(|| {
            if options
                .token
                .as_deref()
                .is_some_and(|token| !token.is_empty())
            {
                Some(AuthMethod::Token)
            } else if options.api_key.is_some() || options.app_key.is_some() {
                Some(AuthMethod::ApiKeys)
            } else {
                None
            }
        });
        let global_method = files.global.auth.as_ref().map(|auth| auth.method);
        let method = cli_method.or(global_method);
        let source = if cli_method.is_some() {
            "CLI"
        } else if global_method.is_some() {
            "global config"
        } else {
            "legacy precedence"
        };
        (method, source)
    }

    fn resolve(
        files: ConfigFiles,
        mut environment: ConfigFile,
        options: ConfigOptions,
    ) -> Result<Self> {
        let (method, source) = Self::select_method(&files, &options);
        if let Some(method) = method {
            let conflict = match method {
                AuthMethod::ApiKeys => options.token.is_some(),
                AuthMethod::Token => options.api_key.is_some() || options.app_key.is_some(),
                AuthMethod::Oauth => {
                    options.token.is_some()
                        || options.api_key.is_some()
                        || options.app_key.is_some()
                }
            };
            if conflict && (options.auth_method.is_some() || method == AuthMethod::Oauth) {
                return Err(DatadogError::InvalidInput(
                    "Credential flags must match --auth-method".into(),
                ));
            }
        }
        if options.pup_org.is_some() && method != Some(AuthMethod::Oauth) {
            return Err(DatadogError::InvalidInput(
                "--pup-org requires OAuth authentication".into(),
            ));
        }
        let pup_org = options.pup_org.or_else(|| {
            files
                .global
                .auth
                .as_ref()
                .and_then(|auth| auth.pup_org.clone())
        });
        if method == Some(AuthMethod::Oauth) {
            if let Some(org) = &pup_org {
                crate::auth::validate_pup_org(org)?;
            }
            if options.site.is_none() && environment.site.is_none() {
                let trusted_site = files.global.site.as_deref().unwrap_or(DEFAULT_SITE);
                if files
                    .project
                    .site
                    .as_deref()
                    .is_some_and(|site| site != trusted_site)
                {
                    return Err(DatadogError::InvalidInput(
                        "Project site conflicts with the OAuth site; select the site explicitly with --site or DD_SITE".into(),
                    ));
                }
            }
        }
        let mut file = files.merged(method);
        environment.retain_credentials(method);
        file.overlay(environment);
        file.overlay(ConfigFile {
            api_key: options.api_key,
            app_key: options.app_key,
            token: options.token,
            site: options.site,
            ..ConfigFile::default()
        });
        Ok(Self {
            file,
            method,
            source,
            pup_org,
        })
    }

    async fn execute(self) -> Result<Config> {
        if self.method != Some(AuthMethod::Oauth) {
            return Config::resolve_with_method(self.file, self.method);
        }
        let (site, defaults, network) = Config::resolve_settings(&self.file)?;
        let org = self.pup_org.as_deref().ok_or_else(|| {
            DatadogError::InvalidInput(
                "OAuth requires [auth].pup_org in the global config.toml or --pup-org".into(),
            )
        })?;
        let token = crate::auth::pup_token(&site, org).await?;
        Ok(Config {
            credentials: Credentials::Token(token),
            site,
            defaults,
            network,
        })
    }
}

impl Config {
    pub fn load(
        cli_api_key: Option<String>,
        cli_app_key: Option<String>,
        cli_token: Option<String>,
        cli_site: Option<String>,
    ) -> Result<Self> {
        let plan = ConfigPlan::load(ConfigOptions {
            api_key: cli_api_key,
            app_key: cli_app_key,
            token: cli_token,
            site: cli_site,
            ..ConfigOptions::default()
        })?;
        Self::resolve_with_method(plan.file, plan.method)
    }

    pub async fn load_with_options(options: ConfigOptions) -> Result<Self> {
        Self::load_with_site_check(options, |_| Ok(())).await
    }

    pub(crate) async fn load_with_site_check(
        options: ConfigOptions,
        check: impl FnOnce(&str) -> Result<()>,
    ) -> Result<Self> {
        let plan = ConfigPlan::load(options)?;
        let (site, _, _) = Self::resolve_settings(&plan.file)?;
        check(&site)?;
        plan.execute().await
    }

    pub async fn load_pup(cli_site: Option<String>, org: &str) -> Result<Self> {
        Self::load_with_options(ConfigOptions {
            site: cli_site,
            auth_method: Some(AuthMethod::Oauth),
            pup_org: Some(org.into()),
            ..ConfigOptions::default()
        })
        .await
    }

    #[cfg(test)]
    fn resolve(file: ConfigFile) -> Result<Self> {
        Self::resolve_with_method(file, None)
    }

    fn resolve_with_method(file: ConfigFile, method: Option<AuthMethod>) -> Result<Self> {
        if method == Some(AuthMethod::Oauth) {
            return Err(DatadogError::InvalidInput(
                "OAuth requires asynchronous Config::load_with_options".into(),
            ));
        }
        let (site, defaults, network) = Self::resolve_settings(&file)?;
        if method == Some(AuthMethod::Token) {
            let token = file
                .token
                .filter(|token| !token.is_empty())
                .ok_or_else(|| {
                    DatadogError::AuthError(
                        "Token authentication requires token in config, DD_TOKEN, or --token"
                            .into(),
                    )
                })?;
            return Ok(Self {
                credentials: Credentials::Token(token),
                site,
                defaults,
                network,
            });
        }
        let mut file = file;
        if method == Some(AuthMethod::ApiKeys) {
            file.token = None;
        }
        let credentials = match file.token.filter(|t| !t.is_empty()) {
            Some(token) => Credentials::Token(token),
            None => {
                let api_key = file.api_key.filter(|k| !k.is_empty());
                let app_key = file.app_key.filter(|k| !k.is_empty());
                match (api_key, app_key) {
                    (Some(api_key), Some(app_key)) => Credentials::Keys { api_key, app_key },
                    _ => {
                        return Err(DatadogError::AuthError(
                            "credentials required: set a bearer token (--token, \
                             DD_TOKEN env, or `token` in config) or both api_key and app_key \
                             (--api-key/--app-key, DD_API_KEY/DD_APP_KEY env, or config file)"
                                .into(),
                        ));
                    }
                }
            }
        };

        Ok(Self {
            credentials,
            site,
            defaults,
            network,
        })
    }

    fn resolve_settings(file: &ConfigFile) -> Result<(String, Defaults, Network)> {
        let site = file
            .site
            .clone()
            .unwrap_or_else(|| DEFAULT_SITE.to_string());
        if !VALID_SITES.contains(&site.as_str()) {
            return Err(DatadogError::InvalidInput(format!(
                "Invalid site: {}. Valid sites: {}",
                site,
                VALID_SITES.join(", ")
            )));
        }

        let base = Defaults::default();
        let defaults = Defaults {
            format: file.defaults.format.clone().unwrap_or(base.format),
            tag_filter: file.defaults.tag_filter.clone(),
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

        let now = std::time::Instant::now();
        if [network.timeout_secs, network.total_timeout_secs]
            .into_iter()
            .any(|seconds| {
                now.checked_add(std::time::Duration::from_secs(seconds))
                    .is_none()
            })
        {
            return Err(DatadogError::InvalidInput(
                "network timeouts exceed the platform clock range".into(),
            ));
        }

        Ok((site, defaults, network))
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
        env::home_dir().map(|home| home.join(".config/datadog-cli/config.toml"))
    }

    #[cfg(unix)]
    fn check_permissions(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(path)?;
        let mode = metadata.permissions().mode();

        if mode & 0o077 != 0 {
            crate::diagnostics::write(
                &mut std::io::stderr().lock(),
                format_args!(
                    "Warning: Config has insecure permissions: {:o}. Run: chmod 600 {}",
                    mode,
                    path.display()
                ),
            )?;
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
# Or authenticate with a bearer token instead of the key pair:
# token = "your-bearer-token"
site = "datadoghq.com"

# Optional explicit authentication selection (global config only):
# [auth]
# method = "api-keys"
# Use method = "oauth" with pup_org after logging in through Pup.
# pup_org = "cli-read"

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
        Self::show_with_options(ConfigOptions {
            api_key,
            app_key,
            token,
            site,
            ..ConfigOptions::default()
        })
    }

    pub fn show_with_options(options: ConfigOptions) -> Result<String> {
        let plan = ConfigPlan::load(options)?;
        let file = plan.file;
        let path = Self::active_config_path()?.display().to_string();

        let defaults = Defaults::default();
        let network = Network::default();

        Ok(format!(
            "Config: {}\n\
             Global config: {}\n\
             Authentication: {}\n\
             Auth source: {}\n\
             Pup session: {}\n\
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
            plan.method
                .map_or("legacy (token/key precedence)", AuthMethod::as_str),
            plan.source,
            if plan.method == Some(AuthMethod::Oauth) {
                plan.pup_org.as_deref().unwrap_or("(not set)")
            } else {
                "(not selected)"
            },
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
        let merged = Config::resolve(
            ConfigFiles::load_paths(Some(&global), Some(&project))
                .unwrap()
                .merged(None),
        )
        .unwrap();
        assert_eq!(merged.site, "datadoghq.eu");
        assert_eq!(merged.defaults.format, "jsonl");
        fs::write(&global, "api_key = 'secret-value' broken").unwrap();
        let error = ConfigFiles::load_paths(Some(&global), Some(&project))
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
    fn large_network_timeouts_follow_the_platform_clock_range() {
        let seconds = i64::MAX as u64;
        let duration = std::time::Duration::from_secs(seconds);
        let representable = std::time::Instant::now().checked_add(duration).is_some();
        for key in ["timeout_secs", "total_timeout_secs"] {
            let mut file = file_with_keys();
            file.overlay(toml::from_str(&format!("[network]\n{key} = {seconds}")).unwrap());
            match Config::resolve(file) {
                Ok(config) => {
                    assert!(
                        representable,
                        "Accepted an unrepresentable timeout for {key}"
                    );
                    let actual = if key == "timeout_secs" {
                        config.network.timeout_secs
                    } else {
                        config.network.total_timeout_secs
                    };
                    assert_eq!(actual, seconds);
                }
                Err(error) => {
                    assert!(
                        !representable,
                        "Rejected a representable timeout for {key}: {error}"
                    );
                    assert!(error.to_string().contains("platform clock range"));
                }
            }
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
    fn auth_plan(
        global: &str,
        project: &str,
        environment: &str,
        options: ConfigOptions,
    ) -> Result<ConfigPlan> {
        ConfigPlan::resolve(
            ConfigFiles {
                global: toml::from_str(global).unwrap(),
                project: toml::from_str(project).unwrap(),
            },
            toml::from_str(environment).unwrap(),
            options,
        )
    }

    #[test]
    fn explicit_authentication_filters_other_methods_before_merging() {
        let keys = auth_plan(
            "api_key='test123'\napp_key='app456'\ntoken='unused'\n[auth]\nmethod='api-keys'",
            "token='project-unused'",
            "token='environment-unused'",
            ConfigOptions::default(),
        )
        .unwrap();
        assert_eq!(keys.method, Some(AuthMethod::ApiKeys));
        assert_keys(&Config::resolve_with_method(keys.file, keys.method).unwrap());
        let token = auth_plan(
            "token='selected'\n[auth]\nmethod='token'",
            "api_key='project-unused'\napp_key='project-unused'",
            "api_key='environment-unused'\napp_key='environment-unused'",
            ConfigOptions::default(),
        )
        .unwrap();
        assert!(
            matches!(Config::resolve_with_method(token.file, token.method).unwrap().credentials,
            Credentials::Token(value) if value == "selected")
        );
        let oauth = auth_plan(
            "token='unused'\n[auth]\nmethod='oauth'\npup_org='selected'",
            "api_key='unused'\napp_key='unused'",
            "token='unused'",
            ConfigOptions::default(),
        )
        .unwrap();
        assert_eq!(oauth.method, Some(AuthMethod::Oauth));
        assert_eq!(oauth.pup_org.as_deref(), Some("selected"));
        assert!(
            oauth.file.token.is_none()
                && oauth.file.api_key.is_none()
                && oauth.file.app_key.is_none()
        );
    }

    #[test]
    fn explicit_authentication_never_uses_another_methods_credentials() {
        for source in [
            "token='other'\n[auth]\nmethod='api-keys'",
            "api_key='only-one-key'\ntoken='other'\n[auth]\nmethod='api-keys'",
            "api_key='k'\napp_key='a'\n[auth]\nmethod='token'",
            "token=''\napi_key='k'\napp_key='a'\n[auth]\nmethod='token'",
        ] {
            let plan = auth_plan(source, "", "", ConfigOptions::default()).unwrap();
            assert_eq!(
                Config::resolve_with_method(plan.file, plan.method)
                    .unwrap_err()
                    .exit_code(),
                3
            );
        }
    }

    #[test]
    fn cli_authentication_overrides_global_defaults_and_rejects_conflicts() {
        let global = "api_key='test123'\napp_key='app456'\ntoken='global-token'\n[auth]\nmethod='oauth'\npup_org='global-session'";
        for options in [
            ConfigOptions {
                auth_method: Some(AuthMethod::ApiKeys),
                ..ConfigOptions::default()
            },
            ConfigOptions {
                api_key: Some("test123".into()),
                ..ConfigOptions::default()
            },
        ] {
            let plan = auth_plan(global, "", "", options).unwrap();
            assert_eq!(plan.source, "CLI");
            assert_keys(&Config::resolve_with_method(plan.file, plan.method).unwrap());
        }
        let plan = auth_plan(
            global,
            "",
            "",
            ConfigOptions {
                token: Some("cli-token".into()),
                ..ConfigOptions::default()
            },
        )
        .unwrap();
        assert!(
            matches!(Config::resolve_with_method(plan.file, plan.method).unwrap().credentials, Credentials::Token(value) if value=="cli-token")
        );
        for options in [
            ConfigOptions {
                auth_method: Some(AuthMethod::ApiKeys),
                token: Some("unused".into()),
                ..ConfigOptions::default()
            },
            ConfigOptions {
                auth_method: Some(AuthMethod::Token),
                api_key: Some("unused".into()),
                ..ConfigOptions::default()
            },
            ConfigOptions {
                auth_method: Some(AuthMethod::Oauth),
                token: Some("unused".into()),
                ..ConfigOptions::default()
            },
            ConfigOptions {
                token: Some(String::new()),
                ..ConfigOptions::default()
            },
        ] {
            assert_eq!(
                auth_plan(global, "", "", options)
                    .err()
                    .unwrap()
                    .exit_code(),
                1
            );
        }
    }

    #[test]
    fn oauth_site_and_session_require_trusted_selection() {
        let global = "site='datadoghq.eu'\n[auth]\nmethod='oauth'\npup_org='global-session'";
        assert_eq!(
            auth_plan(global, "site='datadoghq.com'", "", ConfigOptions::default())
                .err()
                .unwrap()
                .exit_code(),
            1
        );
        for (environment, options) in [
            ("site='datadoghq.com'", ConfigOptions::default()),
            (
                "",
                ConfigOptions {
                    site: Some("datadoghq.com".into()),
                    ..ConfigOptions::default()
                },
            ),
        ] {
            let plan = auth_plan(global, "site='datadoghq.com'", environment, options).unwrap();
            assert_eq!(plan.file.site.as_deref(), Some("datadoghq.com"));
        }
        let plan = auth_plan(
            global,
            "",
            "",
            ConfigOptions {
                pup_org: Some("cli-session".into()),
                ..ConfigOptions::default()
            },
        )
        .unwrap();
        assert_eq!(plan.pup_org.as_deref(), Some("cli-session"));
        assert!(
            auth_plan(
                "",
                "",
                "",
                ConfigOptions {
                    pup_org: Some("session".into()),
                    ..ConfigOptions::default()
                }
            )
            .is_err()
        );
        assert!(
            auth_plan(
                "[auth]\nmethod='oauth'\npup_org='  '",
                "",
                "",
                ConfigOptions::default()
            )
            .is_err()
        );
    }

    #[test]
    fn auth_schema_and_project_scope_are_strict() {
        for source in [
            "[auth]\nmethod='unknown'",
            "[auth]\npup_org='session'",
            "[auth]\nmethod='oauth'\ncommand='not-allowed'",
        ] {
            assert!(toml::from_str::<ConfigFile>(source).is_err());
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".datadog.toml");
        fs::write(&path, "[auth]\nmethod='oauth'\npup_org='session'").unwrap();
        assert_eq!(
            ConfigFiles::load_paths(None, Some(&path))
                .unwrap_err()
                .exit_code(),
            1
        );
    }

    #[test]
    fn legacy_empty_cli_token_still_clears_inherited_token() {
        let plan = auth_plan(
            "api_key='test123'\napp_key='app456'\ntoken='lower'",
            "",
            "",
            ConfigOptions {
                token: Some(String::new()),
                ..ConfigOptions::default()
            },
        )
        .unwrap();
        assert_eq!(plan.method, None);
        assert_keys(&Config::resolve_with_method(plan.file, plan.method).unwrap());
    }
}
