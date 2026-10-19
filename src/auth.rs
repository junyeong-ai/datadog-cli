use crate::error::{DatadogError, Result};
use std::fs::{File, TryLockError};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};

const MAX_TOKEN_BYTES: u64 = 16 * 1024;
const PROVIDER_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) fn validate_pup_org(org: &str) -> Result<()> {
    if org.trim().is_empty() || org.contains('\0') {
        return Err(DatadogError::InvalidInput(
            "pup_org must name a nonempty Pup OAuth session".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn pup_token(site: &str, org: &str) -> Result<String> {
    validate_pup_org(org)?;
    let mut command = Command::new("pup");
    command
        .args(["auth", "token"])
        .arg(format!("--org={org}"))
        .env("DD_SITE", site)
        .env("DD_ORG", org)
        .env_remove("DD_ACCESS_TOKEN")
        .env_remove("DD_TOKEN")
        .env_remove("DD_API_KEY")
        .env_remove("DD_APP_KEY");
    let lock_path = crate::config::Config::global_config_path()
        .ok_or_else(|| DatadogError::AuthError("Cannot determine the OAuth lock directory".into()))?
        .with_file_name(format!(".pup-{site}.lock"));
    token_from_command_locked(command, &lock_path, PROVIDER_TIMEOUT).await
}

async fn provider_lock(path: &Path) -> Result<File> {
    let parent = path.parent().ok_or_else(|| {
        DatadogError::AuthError("Cannot determine the OAuth lock directory".into())
    })?;
    std::fs::create_dir_all(parent)
        .map_err(|_| DatadogError::AuthError("Cannot create the OAuth lock directory".into()))?;
    let mut options = File::options();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(path)
        .map_err(|_| DatadogError::AuthError("Cannot open the OAuth provider lock".into()))?;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) => tokio::time::sleep(Duration::from_millis(25)).await,
            Err(TryLockError::Error(_)) => {
                return Err(DatadogError::AuthError(
                    "Cannot lock the OAuth provider".into(),
                ));
            }
        }
    }
}

async fn token_from_command_locked(
    command: Command,
    path: &Path,
    timeout: Duration,
) -> Result<String> {
    let started = tokio::time::Instant::now();
    let _guard = tokio::time::timeout(timeout, provider_lock(path))
        .await
        .map_err(|_| {
            DatadogError::AuthError("Pup authentication exceeded its time limit".into())
        })??;
    let remaining = timeout
        .checked_sub(started.elapsed())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| {
            DatadogError::AuthError("Pup authentication exceeded its time limit".into())
        })?;
    token_from_command(command, remaining).await
}

async fn token_from_command(mut command: Command, timeout: Duration) -> Result<String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| {
            DatadogError::AuthError(
                "Pup 2.0+ must be installed on PATH for --auth-provider pup".into(),
            )
        })?;
    let result = tokio::time::timeout(timeout, read_token(&mut child))
        .await
        .unwrap_or_else(|_| {
            Err(DatadogError::AuthError(
                "Pup authentication exceeded its time limit".into(),
            ))
        });
    if result.is_err() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}

async fn read_token(child: &mut Child) -> Result<String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| DatadogError::AuthError("Cannot read Pup credentials".into()))?;
    let mut output = Vec::new();
    stdout
        .take(MAX_TOKEN_BYTES + 1)
        .read_to_end(&mut output)
        .await
        .map_err(|_| DatadogError::AuthError("Cannot read Pup credentials".into()))?;
    if output.len() as u64 > MAX_TOKEN_BYTES {
        return Err(DatadogError::AuthError(
            "Pup returned an oversized credential response".into(),
        ));
    }
    let status = child
        .wait()
        .await
        .map_err(|_| DatadogError::AuthError("Cannot wait for Pup authentication".into()))?;
    if !status.success() {
        return Err(DatadogError::AuthError(
            "Pup authentication failed; log in to the selected site and named session with pup auth login"
                .into(),
        ));
    }
    let token = std::str::from_utf8(&output)
        .ok()
        .map(str::trim)
        .filter(|token| {
            let body = token.trim_end_matches('=');
            !body.is_empty()
                && body.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'+' | b'/')
                })
        })
        .ok_or_else(|| {
            DatadogError::AuthError("Pup did not return a single nonempty bearer token".into())
        })?;
    Ok(token.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn provider_deadlines_and_output_limits_kill_and_reap_the_child() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("provider.rs");
        let executable = root
            .path()
            .join(format!("provider{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(
            &source,
            r#"
use std::io::Write;
fn main() {
    std::fs::write(std::env::args_os().nth(1).unwrap(), std::process::id().to_string()).unwrap();
    if std::env::args().nth(2).as_deref() == Some("serialized") {
        let marker = std::env::args_os().nth(3).unwrap();
        let guard = std::fs::File::create_new(&marker).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(75));
        drop(guard);
        std::fs::remove_file(marker).unwrap();
        println!("fixture-token");
        return;
    }
    if std::env::args().nth(2).as_deref() == Some("oversize") {
        std::io::stdout().write_all(&vec![b'x'; 32768]).unwrap();
    }
    std::thread::sleep(std::time::Duration::from_secs(30));
}
"#,
        )
        .unwrap();
        assert!(
            std::process::Command::new("rustc")
                .args(["--crate-name", "provider_fixture"])
                .arg(&source)
                .arg("-o")
                .arg(&executable)
                .status()
                .unwrap()
                .success()
        );
        for mode in ["wait", "oversize"] {
            let pid_file = root.path().join(mode);
            let mut command = Command::new(&executable);
            command.arg(&pid_file).arg(mode);
            let error = token_from_command(command, Duration::from_secs(3))
                .await
                .unwrap_err();
            assert_eq!(error.exit_code(), 3);
            let pid: u32 = std::fs::read_to_string(pid_file).unwrap().parse().unwrap();
            #[cfg(unix)]
            assert_eq!(unsafe { libc::kill(pid as libc::pid_t, 0) }, -1);
            #[cfg(windows)]
            let _ = pid;
        }
        let lock = root.path().join("provider.lock");
        let marker = root.path().join("critical-section");
        let commands = ["first", "second", "third"].map(|name| {
            let mut command = Command::new(&executable);
            command
                .arg(root.path().join(name))
                .arg("serialized")
                .arg(&marker);
            command
        });
        let [first, second, third] = commands;
        let results = tokio::join!(
            token_from_command_locked(first, &lock, Duration::from_secs(3)),
            token_from_command_locked(second, &lock, Duration::from_secs(3)),
            token_from_command_locked(third, &lock, Duration::from_secs(3)),
        );
        assert_eq!(results.0.unwrap(), "fixture-token");
        assert_eq!(results.1.unwrap(), "fixture-token");
        assert_eq!(results.2.unwrap(), "fixture-token");
    }

    #[tokio::test]
    async fn provider_lock_timeout_does_not_start_a_credential_process() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("provider.lock");
        let guard = provider_lock(&path).await.unwrap();
        let error = token_from_command_locked(
            Command::new(root.path().join("must-not-start")),
            &path,
            Duration::from_millis(50),
        )
        .await
        .unwrap_err();
        assert_eq!(error.exit_code(), 3);
        assert!(error.to_string().contains("time limit"));
        drop(guard);
        let guard = provider_lock(&path).await.unwrap();
        drop(guard);
    }
}
