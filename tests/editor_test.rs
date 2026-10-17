use std::fs;
use std::process::Command;

#[test]
fn config_edit_passes_editor_arguments_and_preserves_process_results() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let source = root.join("editor.rs");
    let executable = root.join(format!(
        "editor with spaces{}",
        std::env::consts::EXE_SUFFIX
    ));
    let output = root.join("arguments.txt");
    fs::write(&source, r#"
use std::io::Write;
fn main() {
    let mut output = std::fs::File::create(std::env::var_os("TEST_EDITOR_OUTPUT").unwrap()).unwrap();
    for arg in std::env::args().skip(1) { writeln!(output, "{arg}").unwrap(); }
    if std::env::var_os("TEST_EDITOR_FAIL").is_some() { std::process::exit(7); }
}
"#).unwrap();
    assert!(
        Command::new("rustc")
            .args(["--crate-name", "editor_fixture", "--edition=2024"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .status()
            .unwrap()
            .success()
    );

    let project = root.join("project with spaces & 한글");
    fs::create_dir(&project).unwrap();
    let config = project.join(".datadog.toml");
    fs::write(&config, "").unwrap();
    let editor = format!(
        "\"{}\" --wait \"two words\" \"\" \"$literal\" \"a;b\"",
        executable.display()
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_datadog-cli"));
    command
        .args(["config", "edit"])
        .current_dir(&project)
        .env("EDITOR", editor)
        .env("TEST_EDITOR_OUTPUT", &output);
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let contents = fs::read_to_string(&output).unwrap();
    let arguments: Vec<&str> = contents.lines().collect();
    assert_eq!(arguments.len(), 6);
    assert_eq!(
        &arguments[..5],
        &["--wait", "two words", "", "$literal", "a;b"]
    );
    assert_eq!(
        fs::canonicalize(arguments[5]).unwrap(),
        fs::canonicalize(&config).unwrap()
    );

    command.env("TEST_EDITOR_FAIL", "1");
    let result = command.output().unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("Editor exited with error"));
    for invalid in ["", "  ", "\"\""] {
        fs::remove_file(&output).unwrap();
        let result = command.env("EDITOR", invalid).output().unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(!output.exists());
        fs::write(&output, "").unwrap();
    }
}
