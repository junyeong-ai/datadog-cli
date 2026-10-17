use std::fs;
use std::process::Command;

#[test]
fn pup_authentication_is_explicit_bounded_and_does_not_fall_back() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("pup.rs");
    let program = root
        .path()
        .join(format!("pup{}", std::env::consts::EXE_SUFFIX));
    fs::write(&source, r#"
use std::io::Write;
fn main() {
    std::fs::write(std::env::var_os("TEST_PUP_RECORD").unwrap(), "invoked").unwrap();
    assert_eq!(std::env::args().skip(1).collect::<Vec<_>>(), ["auth", "token", "--org=team work 한글;$literal"]);
    assert_eq!(std::env::var("DD_SITE").unwrap(), "datadoghq.eu");
    assert!(std::env::var_os("DD_ACCESS_TOKEN").is_none());
    assert_eq!(std::env::var("DD_ORG").unwrap(), "team work 한글;$literal");
    for key in ["DD_API_KEY", "DD_APP_KEY", "DD_TOKEN"] { assert!(std::env::var_os(key).is_none()); }
    let case = std::env::var("TEST_PUP_CASE").unwrap();
    match case.as_str() {
        "failed" => { eprintln!("fixture-secret-stderr"); println!("fixture-secret-stdout"); std::process::exit(7); },
        "empty" => println!(),
        "multiline" => println!("fixture-secret\nextra"),
        "invalid_token" => println!("invalid<token>"),
        "invalid_utf8" => std::io::stdout().write_all(&[255]).unwrap(),
        "oversize" => std::io::stdout().write_all(&vec![b'x';32768]).unwrap(),
        _ => print!("fixture-oauth-access-token\r\n"),
    }
}
"#).unwrap();
    assert!(
        Command::new("rustc")
            .args(["--crate-name", "pup_fixture"])
            .arg(&source)
            .arg("-o")
            .arg(&program)
            .status()
            .unwrap()
            .success()
    );
    let project = root.path().join("project");
    fs::create_dir(&project).unwrap();
    let config = project.join(".datadog.toml");
    fs::write(&config, "api_key='fallback-api-key'\napp_key='fallback-app-key'\ntoken='fallback-token'\nsite='datadoghq.eu'\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let paths = std::env::join_paths(
        std::iter::once(root.path().to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let record = root.path().join("record");
    for case in [
        "success",
        "failed",
        "empty",
        "multiline",
        "invalid_token",
        "invalid_utf8",
        "oversize",
    ] {
        let result = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "pup_provider_child", "--nocapture"])
            .current_dir(&project)
            .env("HOME", root.path())
            .env("USERPROFILE", root.path())
            .env("PATH", &paths)
            .env("TEST_PUP_CASE", case)
            .env("TEST_PUP_RECORD", &record)
            .env("DD_ACCESS_TOKEN", "ambient-token-must-not-be-used")
            .env("DD_ORG", "ambient-org-must-not-be-used")
            .env_remove("DD_SITE")
            .env("DD_TOKEN", "fallback-environment-token")
            .env("DD_API_KEY", "fallback-environment-api-key")
            .env("DD_APP_KEY", "fallback-environment-app-key")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(record.exists());
        fs::remove_file(&record).unwrap();
    }
    for args in [
        vec![
            "--auth-provider",
            "pup",
            "--pup-org",
            "team",
            "--token",
            "fixture-secret",
            "metrics",
            "q",
        ],
        vec![
            "--auth-provider",
            "pup",
            "--pup-org",
            "team",
            "--api-key",
            "fixture-secret",
            "metrics",
            "q",
        ],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_datadog-cli"))
            .args(args)
            .current_dir(&project)
            .env("HOME", root.path())
            .env("USERPROFILE", root.path())
            .env("PATH", &paths)
            .env("TEST_PUP_RECORD", &record)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(!record.exists());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("fixture-secret"));
    }
    for args in [
        vec!["--auth-provider", "pup", "metrics", "q"],
        vec!["--pup-org", "team", "metrics", "q"],
        vec![
            "--auth-provider",
            "pup",
            "--pup-org",
            "team",
            "--site",
            "invalid.example",
            "metrics",
            "q",
        ],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_datadog-cli"))
            .args(args)
            .current_dir(&project)
            .env("HOME", root.path())
            .env("USERPROFILE", root.path())
            .env("PATH", &paths)
            .env("TEST_PUP_RECORD", &record)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(!record.exists());
    }
    let global_directory = root.path().join(".config/datadog-cli");
    fs::create_dir_all(&global_directory).unwrap();
    let global_config = global_directory.join("config.toml");
    fs::write(&global_config, "site='datadoghq.eu'\ntoken='unused-global-secret'\n[auth]\nmethod='oauth'\npup_org='team work 한글;$literal'\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&global_config, fs::Permissions::from_mode(0o600)).unwrap();
    }
    for (case, arguments, expected_exit, expected_provider) in [
        ("failed", vec!["config", "show"], 0, false),
        (
            "success",
            vec!["metrics", "q", "--from", "definitely-invalid"],
            1,
            true,
        ),
        (
            "failed",
            vec!["metrics", "q", "--from", "definitely-invalid"],
            3,
            true,
        ),
        (
            "failed",
            vec![
                "--auth-method",
                "api-keys",
                "metrics",
                "q",
                "--from",
                "definitely-invalid",
            ],
            1,
            false,
        ),
        (
            "failed",
            vec![
                "--auth-method",
                "token",
                "metrics",
                "q",
                "--from",
                "definitely-invalid",
            ],
            1,
            false,
        ),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_datadog-cli"))
            .args(arguments)
            .current_dir(&project)
            .env("HOME", root.path())
            .env("USERPROFILE", root.path())
            .env("PATH", &paths)
            .env_remove("DD_SITE")
            .env("DD_TOKEN", "ambient-secret")
            .env("DD_API_KEY", "ambient-api")
            .env("DD_APP_KEY", "ambient-app")
            .env("TEST_PUP_RECORD", &record)
            .env("TEST_PUP_CASE", case)
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(expected_exit),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(record.exists(), expected_provider);
        let output = format!(
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            !output.contains("unused-global-secret")
                && !output.contains("ambient-secret")
                && !output.contains("fixture-secret")
        );
        if expected_exit == 0 {
            assert!(output.contains("Authentication: oauth"));
        }
        if record.exists() {
            fs::remove_file(&record).unwrap();
        }
    }
}

#[tokio::test]
async fn pup_provider_child() {
    let Ok(case) = std::env::var("TEST_PUP_CASE") else {
        return;
    };
    use datadog_cli::config::Config;
    use datadog_cli::datadog::DatadogClient;
    use wiremock::matchers::{header, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let result = Config::load_pup(Some("datadoghq.eu".into()), "team work 한글;$literal").await;
    if case != "success" {
        let error = result.unwrap_err();
        assert_eq!(error.exit_code(), 3);
        assert!(!error.to_string().contains("fixture-secret"));
        return;
    }
    let config = result.unwrap();
    assert!(!format!("{config:?}").contains("fixture-oauth-access-token"));
    let server = MockServer::start().await;
    Mock::given(path("/api/v1/query"))
        .and(header("authorization", "Bearer fixture-oauth-access-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"status":"ok","series":[],"query":"q"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = DatadogClient::with_base_url(&config, server.uri()).unwrap();
    client.query_metrics("q", 0, 1).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    assert!(!requests[0].headers.contains_key("dd-api-key"));
    assert!(!requests[0].headers.contains_key("dd-application-key"));
}
