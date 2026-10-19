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
