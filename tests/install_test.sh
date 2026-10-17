#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
work_dir=$(mktemp -d "${TMPDIR:-/tmp}/datadog-cli-test.XXXXXX")
trap 'rm -rf "$work_dir"' EXIT
fixture="$work_dir/source checkout"
mkdir -p "$fixture/scripts" "$fixture/src" "$fixture/.claude/skills/datadog-query" "$work_dir/tmp"
cp "$repo_root/scripts/install.sh" "$fixture/scripts/"
cp "$repo_root/rust-toolchain.toml" "$fixture/"
cp "$repo_root/.claude/skills/datadog-query/"*.md "$fixture/.claude/skills/datadog-query/"
cat > "$fixture/Cargo.toml" <<'TOML'
[package]
name = "datadog-cli"
version = "0.4.0"
edition = "2024"
TOML
cat > "$fixture/src/main.rs" <<'RS'
fn main() {
    println!("datadog-cli {}", env!("CARGO_PKG_VERSION"));
}
RS
export CARGO_NET_OFFLINE=true
export CARGO_BUILD_TARGET=wasm32-unknown-unknown
export TMPDIR="$work_dir/tmp"
export INSTALL_DIR="$work_dir/bin"
export DATADOG_CLI_SKILL_DIR="$work_dir/skill"
unset DATADOG_CLI_VERSION CARGO_TARGET_DIR
cargo generate-lockfile --manifest-path "$fixture/Cargo.toml"
mkdir -p "$fixture/target/release"
printf '#!/usr/bin/env bash\nprintf "stale binary\\n"\n' > "$fixture/target/release/datadog-cli"
chmod +x "$fixture/target/release/datadog-cli"

for scenario in default environment configuration; do
    case "$scenario" in
        environment) export CARGO_TARGET_DIR="$work_dir/shared target" ;;
        configuration)
            unset CARGO_TARGET_DIR
            mkdir -p "$fixture/.cargo"
            printf '[build]\ntarget-dir = "%s"\n' "$work_dir/configured target" > "$fixture/.cargo/config.toml"
            ;;
    esac
    if ! bash "$fixture/scripts/install.sh" --source </dev/null > "$work_dir/install.log" 2>&1; then
        cat "$work_dir/install.log" >&2
        exit 1
    fi
    test "$("$INSTALL_DIR/datadog-cli" --version)" = "datadog-cli 0.4.0"
    for name in SKILL.md reference.md examples.md; do
        cmp "$fixture/.claude/skills/datadog-query/$name" "$DATADOG_CLI_SKILL_DIR/$name"
    done
    for path in "$TMPDIR"/datadog-cli-install.*; do
        test ! -e "$path"
    done
    echo "Source install ($scenario target directory): passed"
done

printf 'invalid Rust\n' > "$fixture/src/main.rs"
if bash "$fixture/scripts/install.sh" --source </dev/null > "$work_dir/install.log" 2>&1; then
    echo "A failed source build must fail installation" >&2
    exit 1
fi
test "$("$INSTALL_DIR/datadog-cli" --version)" = "datadog-cli 0.4.0"
for path in "$TMPDIR"/datadog-cli-install.*; do
    test ! -e "$path"
done
echo "Failed source build preserves the installed binary and cleans staging: passed"

bash "$repo_root/scripts/uninstall.sh" </dev/null > "$work_dir/uninstall.log"
test ! -e "$INSTALL_DIR/datadog-cli"
test -f "$DATADOG_CLI_SKILL_DIR/SKILL.md"
echo "Script uninstall preserves the skill by default: passed"
