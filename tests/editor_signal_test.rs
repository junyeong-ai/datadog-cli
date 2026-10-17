#![cfg(unix)]

use std::fs;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

struct Session(Child);
impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            libc::kill(-(self.0.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = self.0.wait();
    }
}

fn wait_until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "editor lifecycle did not complete"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn editor_owns_terminal_interrupts_until_it_exits_and_is_reaped() {
    for mode in ["exit", "signal", "continue"] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::write(root.join(".datadog.toml"), "").unwrap();
        let script = root.join("editor.sh");
        fs::write(
            &script,
            r#"
if [ "$EDITOR_MODE" = continue ]; then
    trap 'printf interrupted > interrupted' INT
elif [ "$EDITOR_MODE" = exit ]; then
    trap 'exit 130' INT
fi
printf ready > ready
while [ ! -f finish ]; do sleep 0.05; done
"#,
        )
        .unwrap();
        let command = format!("/bin/bash \"{}\"", script.display());
        let mut session = Session(
            Command::new(env!("CARGO_BIN_EXE_datadog-cli"))
                .args(["config", "edit"])
                .current_dir(root)
                .env("EDITOR", command)
                .env("EDITOR_MODE", mode)
                .env("RUST_LOG", "warn")
                .process_group(0)
                .spawn()
                .unwrap(),
        );
        wait_until(|| root.join("ready").exists());
        assert_eq!(
            unsafe { libc::kill(-(session.0.id() as libc::pid_t), libc::SIGINT) },
            0
        );
        if mode == "continue" {
            wait_until(|| root.join("interrupted").exists());
            assert!(session.0.try_wait().unwrap().is_none());
            fs::write(root.join("finish"), "").unwrap();
        }
        let mut status = None;
        wait_until(|| {
            status = session.0.try_wait().unwrap();
            status.is_some()
        });
        assert_eq!(
            status.unwrap().code(),
            Some(if mode == "continue" { 0 } else { 130 })
        );
    }
}
