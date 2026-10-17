#![cfg(unix)]

use std::fs;
use std::os::fd::OwnedFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::process::{Command, Output, Stdio};

fn run_with_closed_stderr(directory: &std::path::Path, arguments: &[&str]) -> Output {
    let (reader, writer) = UnixStream::pair().unwrap();
    drop(reader);
    let writer: OwnedFd = writer.into();
    Command::new(env!("CARGO_BIN_EXE_datadog-cli"))
        .args(arguments)
        .current_dir(directory)
        .env("HOME", directory)
        .env("RUST_LOG", "off")
        .env_remove("DD_API_KEY")
        .env_remove("DD_APP_KEY")
        .env_remove("DD_TOKEN")
        .env_remove("DD_SITE")
        .env_remove("DD_TAG_FILTER")
        .stderr(Stdio::from(writer))
        .output()
        .unwrap()
}

#[test]
fn closed_diagnostic_streams_preserve_results_and_failure_exit_codes() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join(".datadog.toml");
    fs::write(&config, "token = \"fixture-secret\"\n").unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o644)).unwrap();
    let result = run_with_closed_stderr(directory.path(), &["config", "show"]);
    assert_eq!(result.status.code(), Some(0));
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(output.contains("Token: ***"));
    assert!(!output.contains("fixture-secret"));

    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    let result = run_with_closed_stderr(
        directory.path(),
        &[
            "--site",
            "invalid.example",
            "metrics",
            "avg:system.cpu.user{*}",
        ],
    );
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());

    fs::write(&config, "").unwrap();
    let result = run_with_closed_stderr(directory.path(), &["metrics", "avg:system.cpu.user{*}"]);
    assert_eq!(result.status.code(), Some(3));
    assert!(result.stdout.is_empty());
}
