use serde_json::Value;
use std::process::Command;

#[test]
fn json_runtime_errors_stay_on_stderr_and_keep_process_status() {
    let directory = tempfile::tempdir().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_datadog-cli"))
        .args([
            "--error-format",
            "json",
            "--site",
            "invalid.example",
            "metrics",
            "avg:system.cpu.user{*}",
        ])
        .current_dir(directory.path())
        .env("HOME", directory.path())
        .env("USERPROFILE", directory.path())
        .env("RUST_LOG", "off")
        .env_remove("DD_SITE")
        .env_remove("DD_API_KEY")
        .env_remove("DD_APP_KEY")
        .env_remove("DD_TOKEN")
        .env_remove("DD_TAG_FILTER")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    let error: Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"]["kind"], "invalid_input");
    assert_eq!(error["error"]["exit_code"], 1);
    assert_eq!(String::from_utf8(result.stderr).unwrap().lines().count(), 1);
}
