#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="datadog-cli"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
REPO="junyeong-ai/datadog-cli"
SKILL_NAME="datadog-query"
SKILL_FILES=("SKILL.md" "reference.md" "examples.md")
USER_SKILL_DIR="${DATADOG_CLI_SKILL_DIR:-$HOME/.claude/skills/$SKILL_NAME}"
SCRIPT_PATH="${BASH_SOURCE[0]:-$0}"
ORIGINAL_DIR="$(pwd)"
if SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" 2>/dev/null && pwd -P)"; then
    :
else
    SCRIPT_DIR="$ORIGINAL_DIR"
fi
PROJECT_ROOT="$ORIGINAL_DIR"
SKILL_SOURCE_DIR=""
SKILL_TMP_DIR=""
SKILL_STAGE_DIR=""
BINARY_TMP_DIR=""

if [ -f "$SCRIPT_DIR/../Cargo.toml" ]; then
    PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
fi

PROJECT_SKILL_DIR="$PROJECT_ROOT/.claude/skills/$SKILL_NAME"

cleanup() {
    if [ -n "$SKILL_TMP_DIR" ]; then
        rm -rf "$SKILL_TMP_DIR"
    fi
    if [ -n "$SKILL_STAGE_DIR" ]; then
        rm -rf "$SKILL_STAGE_DIR"
    fi
    if [ -n "$BINARY_TMP_DIR" ]; then
        rm -rf "$BINARY_TMP_DIR"
    fi
    return 0
}

trap cleanup EXIT

prompt_choice() {
    local prompt="$1"
    local default="$2"
    local choice=""

    if [ -t 0 ]; then
        read -r -p "$prompt" choice || choice=""
    else
        choice="$default"
    fi

    echo "${choice:-$default}"
}

display_path() {
    local path="$1"

    if [ "$path" = "$HOME" ]; then
        echo "\$HOME"
    elif [[ "$path" == "$HOME/"* ]]; then
        echo "\$HOME/${path#"$HOME"/}"
    else
        echo "$path"
    fi
}

is_valid_release_version() {
    local version="$1"

    [[ "$version" =~ ^[0-9][0-9A-Za-z._+-]*$ ]]
}

detect_platform() {
    local os
    local arch
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$os" in
        linux) os="unknown-linux-gnu" ;;
        darwin) os="apple-darwin" ;;
        *) echo "Unsupported OS: $os"; exit 1 ;;
    esac

    case "$arch" in
        x86_64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) echo "Unsupported architecture: $arch"; exit 1 ;;
    esac

    echo "${arch}-${os}"
}

get_latest_version() {
    local latest_url

    latest_url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest" 2>/dev/null || true)
    case "$latest_url" in
        */releases/tag/v*)
            latest_url="${latest_url##*/releases/tag/v}"
            echo "${latest_url%%[/?#]*}"
            return 0
            ;;
    esac

    return 1
}

download_binary() {
    local version="$1"
    local target="$2"
    local archive
    local url
    local checksum_url
    local binary_path

    if [ -n "$version" ]; then
        archive="datadog-cli-v${version}-${target}.tar.gz"
        url="https://github.com/$REPO/releases/download/v${version}/${archive}"
    else
        archive="datadog-cli-${target}.tar.gz"
        url="https://github.com/$REPO/releases/latest/download/${archive}"
    fi
    checksum_url="${url}.sha256"

    BINARY_TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/datadog-cli-install.XXXXXX")

    echo "📥 Downloading $archive..." >&2
    if ! (cd "$BINARY_TMP_DIR" && curl -fsSLO "$url"); then
        echo "❌ Download failed" >&2
        rm -rf "$BINARY_TMP_DIR"
        return 1
    fi

    echo "🔐 Verifying checksum..." >&2
    if ! (cd "$BINARY_TMP_DIR" && curl -fsSLO "$checksum_url"); then
        echo "❌ Checksum download failed" >&2
        rm -rf "$BINARY_TMP_DIR"
        return 1
    fi

    if command -v sha256sum >/dev/null; then
        (cd "$BINARY_TMP_DIR" && sha256sum -c "${archive}.sha256") >&2 || {
            rm -rf "$BINARY_TMP_DIR"
            return 1
        }
    elif command -v shasum >/dev/null; then
        (cd "$BINARY_TMP_DIR" && shasum -a 256 -c "${archive}.sha256") >&2 || {
            rm -rf "$BINARY_TMP_DIR"
            return 1
        }
    else
        echo "❌ No checksum tool found" >&2
        rm -rf "$BINARY_TMP_DIR"
        return 1
    fi

    echo "📦 Extracting..." >&2
    if ! (cd "$BINARY_TMP_DIR" && tar -xzf "$archive") >&2; then
        rm -rf "$BINARY_TMP_DIR"
        return 1
    fi
    binary_path="$BINARY_TMP_DIR/$BINARY_NAME"

    if [ ! -x "$binary_path" ]; then
        echo "❌ Archive did not contain executable $BINARY_NAME" >&2
        rm -rf "$BINARY_TMP_DIR"
        return 1
    fi

    echo "$binary_path"
}

build_from_source() {
    if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
        echo "❌ Source build requires running inside a datadog-cli checkout" >&2
        exit 1
    fi

    echo "🔨 Building from source..." >&2
    BINARY_TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/datadog-cli-install.XXXXXX")
    if ! (cd "$PROJECT_ROOT" && cargo install --path . --locked --bin "$BINARY_NAME" \
        --root "$BINARY_TMP_DIR" --no-track --target host-tuple) >&2; then
        echo "❌ Build failed" >&2
        return 1
    fi
}

install_binary() {
    local binary_path="$1"

    mkdir -p "$INSTALL_DIR"
    local staged_binary
    staged_binary=$(mktemp "$INSTALL_DIR/.$BINARY_NAME.XXXXXX")
    if ! cp "$binary_path" "$staged_binary" || ! chmod +x "$staged_binary" || ! mv -f "$staged_binary" "$INSTALL_DIR/$BINARY_NAME"; then
        rm -f "$staged_binary"
        return 1
    fi

    echo "✅ Installed to $INSTALL_DIR/$BINARY_NAME" >&2
}

check_skill_exists() {
    [ -f "$USER_SKILL_DIR/SKILL.md" ]
}

install_skill() {
    echo "📋 Installing skill to $USER_SKILL_DIR" >&2
    mkdir -p "$(dirname "$USER_SKILL_DIR")"
    SKILL_STAGE_DIR=$(mktemp -d "${TMPDIR:-/tmp}/datadog-cli-skill-stage.XXXXXX")
    if ! cp -R "$SKILL_SOURCE_DIR/." "$SKILL_STAGE_DIR/"; then
        return 1
    fi
    rm -rf "$USER_SKILL_DIR"
    mv "$SKILL_STAGE_DIR" "$USER_SKILL_DIR"
    SKILL_STAGE_DIR=""
    echo "   ✅ Skill installed" >&2
}

fetch_skill_files() {
    local ref="$1"
    local base_url="https://raw.githubusercontent.com/$REPO/$ref/.claude/skills/$SKILL_NAME"
    local file

    for file in "${SKILL_FILES[@]}"; do
        if ! curl -fsSL "$base_url/$file" -o "$SKILL_SOURCE_DIR/$file"; then
            return 1
        fi
    done

    return 0
}

prepare_skill_source() {
    local ref="$1"

    if [ "$ref" = "main" ] && [ -f "$PROJECT_SKILL_DIR/SKILL.md" ]; then
        SKILL_SOURCE_DIR="$PROJECT_SKILL_DIR"
        return 0
    fi

    SKILL_TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/datadog-cli-skill.XXXXXX")
    SKILL_SOURCE_DIR="$SKILL_TMP_DIR/$SKILL_NAME"
    mkdir -p "$SKILL_SOURCE_DIR"

    if ! fetch_skill_files "$ref"; then
        rm -rf "$SKILL_TMP_DIR"
        SKILL_TMP_DIR=""
        SKILL_SOURCE_DIR=""
        return 1
    fi

    return 0
}

prompt_skill_installation() {
    local ref="$1"
    local choice

    if ! prepare_skill_source "$ref"; then
        echo "" >&2
        echo "⚠️  Could not fetch $SKILL_NAME skill from $ref; skipping skill installation" >&2
        return 0
    fi

    echo "" >&2
    echo "🤖 Datadog query skill ($ref)" >&2
    if check_skill_exists; then
        choice=$(prompt_choice "Replace installed skill with $ref without a backup? [Y/n]: " "")
        if [[ "$choice" =~ ^[nN]$ ]]; then
            echo "⏭️  Keeping installed skill" >&2
            return 0
        fi
        install_skill
    else
        choice=$(prompt_choice "Install query skill to $USER_SKILL_DIR? [Y/n]: " "")
        if [[ ! "$choice" =~ ^[nN]$ ]]; then
            install_skill
        fi
    fi

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
}

main() {
    echo "🚀 Installing Datadog CLI..." >&2
    echo "" >&2

    local binary_path=""
    local target
    local version
    local skill_ref="main"
    local binary_dir
    local display_install_dir
    local command_name
    local method
    target=$(detect_platform)
    version="${DATADOG_CLI_VERSION:-}"
    version="${version#v}"
    if [ "$version" = "latest" ]; then
        version=""
    fi

    case "${1:-}" in
        --source) method=2 ;;
        --help)
            echo "Usage: install.sh [--source]" >&2
            echo "DATADOG_CLI_VERSION selects a prebuilt release; INSTALL_DIR selects the destination." >&2
            return 0
            ;;
        "")
            echo "Installation method:" >&2
            echo "  [1] Download prebuilt binary" >&2
            echo "  [2] Build this checkout with rust-toolchain.toml" >&2
            method=$(prompt_choice "Choose [1-2] (default: 1): " "1")
            ;;
        *) echo "Unknown option: $1" >&2; return 1 ;;
    esac
    if [ "$#" -gt 1 ]; then
        echo "Usage: install.sh [--source]" >&2
        return 1
    fi

    case "$method" in
        2)
            if [ -n "$version" ]; then
                echo "❌ --source builds the current checkout; DATADOG_CLI_VERSION only selects prebuilt binaries." >&2
                return 1
            fi
            build_from_source
            binary_path="$BINARY_TMP_DIR/bin/$BINARY_NAME"
            ;;
        1|"")
            if ! command -v curl >/dev/null; then
                echo "❌ curl is required for binary installation. Use --source from a checkout to build locally." >&2
                return 1
            fi
            if [ -z "$version" ]; then
                if ! version=$(get_latest_version); then
                    echo "❌ Cannot determine the latest release. Set DATADOG_CLI_VERSION explicitly." >&2
                    return 1
                fi
                version="${version#v}"
            fi
            if ! is_valid_release_version "$version"; then
                echo "❌ Invalid release version: $version" >&2
                return 1
            fi
            skill_ref="v$version"
            echo "Version: $skill_ref" >&2
            binary_path=$(download_binary "$version" "$target") || return 1
            binary_dir="$(dirname "$binary_path")"
            case "$(basename "$binary_dir")" in
                datadog-cli-install.*) BINARY_TMP_DIR="$binary_dir" ;;
            esac
            ;;
        *) echo "❌ Invalid installation method" >&2; return 1 ;;
    esac

    install_binary "$binary_path"

    echo "" >&2
    display_install_dir=$(display_path "$INSTALL_DIR")
    command_name="$BINARY_NAME"
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
        echo "✅ $INSTALL_DIR is in PATH" >&2
        ;;
        *)
        command_name="$display_install_dir/$BINARY_NAME"
        echo "⚠️  $INSTALL_DIR not in PATH" >&2
        echo "" >&2
        echo "Add to shell profile (~/.bashrc, ~/.zshrc):" >&2
        echo "  export PATH=\"$display_install_dir:\$PATH\"" >&2
        ;;
    esac
    echo "" >&2

    if [ -x "$INSTALL_DIR/$BINARY_NAME" ]; then
        echo "Installed version:" >&2
        "$INSTALL_DIR/$BINARY_NAME" --version >&2
        echo "" >&2
    fi

    prompt_skill_installation "$skill_ref"

    echo "" >&2
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
    echo "🎉 Installation Complete!" >&2
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
    echo "" >&2
    echo "Next steps:" >&2
    echo "" >&2
    echo "1. Initialize config:   $command_name config init" >&2
    echo "2. Edit credentials:    $command_name config edit" >&2
    echo "3. Try a query:         $command_name monitors list" >&2
    echo "" >&2
}

main "$@"
