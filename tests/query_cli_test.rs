use std::process::Command;

#[test]
fn discovery_and_invalid_inputs_do_not_load_credentials_or_site_configuration() {
    let binary = env!("CARGO_BIN_EXE_datadog-cli");
    let output = Command::new(binary)
        .args([
            "--site",
            "invalid-site",
            "--auth-method",
            "oauth",
            "--pup-org",
            "nonexistent",
            "capabilities",
            "list",
            "--domain",
            "ci-visibility-pipelines",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let data: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        data["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == "v2.SearchCIAppPipelineEvents")
    );
    let output = Command::new(binary)
        .args([
            "--site",
            "invalid-site",
            "--auth-method",
            "oauth",
            "--pup-org",
            "nonexistent",
            "--error-format",
            "json",
            "query",
            "teams",
            "v2.ListTeams",
            "--input",
            r#"{"query":{"page[size]":"sensitive-value"}}"#,
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic["error"]["kind"], "invalid_input");
    let message = diagnostic["error"]["message"].as_str().unwrap();
    assert!(message.contains("violates the API schema"));
    assert!(!message.contains("sensitive-value"));
}

#[test]
fn mcp_discovery_is_offline_and_unreviewed_tools_are_rejected_before_authentication() {
    let binary = env!("CARGO_BIN_EXE_datadog-cli");
    let output = Command::new(binary)
        .args([
            "--site",
            "invalid-site",
            "--auth-method",
            "oauth",
            "--pup-org",
            "nonexistent",
            "mcp",
            "list",
            "--toolset",
            "dbm",
            "--limit",
            "1",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let list: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(list["data"].as_array().unwrap().len(), 1);
    let output = Command::new(binary)
        .args([
            "--site",
            "invalid-site",
            "--auth-method",
            "oauth",
            "--pup-org",
            "nonexistent",
            "--error-format",
            "json",
            "mcp",
            "call",
            "execute_code",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("individually reviewed"));
    let output = Command::new(binary)
        .args([
            "--site",
            "ddog-gov.com",
            "--auth-method",
            "oauth",
            "--pup-org",
            "nonexistent",
            "--error-format",
            "json",
            "mcp",
            "show",
            "find_datadog_database_instances",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("not supported on this site"));
}
