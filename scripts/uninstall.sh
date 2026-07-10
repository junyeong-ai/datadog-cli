#!/usr/bin/env bash
set -euo pipefail

BINARY_NAME="datadog-cli"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
USER_SKILL_DIR="$HOME/.claude/skills/datadog-query"

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

echo "🗑️  Uninstalling Datadog CLI..."
echo

if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
    rm "$INSTALL_DIR/$BINARY_NAME"
    echo "✅ Removed $INSTALL_DIR/$BINARY_NAME"
else
    echo "⚠️  Binary not found at $INSTALL_DIR/$BINARY_NAME"
fi

echo
choice=$(prompt_choice "Remove global configuration? [y/N]: " "")
if [[ "$choice" =~ ^[yY]$ ]]; then
    if [ -d "$HOME/.config/datadog-cli" ]; then
        rm -rf "$HOME/.config/datadog-cli"
        echo "✅ Removed ~/.config/datadog-cli"
    else
        echo "⚠️  Global config not found"
    fi
fi

echo
if [ -d "$USER_SKILL_DIR" ]; then
    echo "📦 Claude Code skill detected at:"
    echo "   $USER_SKILL_DIR"
    echo
    choice=$(prompt_choice "Remove Claude Code skill? [y/N]: " "")
    if [[ "$choice" =~ ^[yY]$ ]]; then
        timestamp=$(date +%Y%m%d-%H%M%S)
        backup_dir="$USER_SKILL_DIR.bak-$timestamp"

        echo "📦 Creating backup: $backup_dir"
        cp -r "$USER_SKILL_DIR" "$backup_dir"

        rm -rf "$USER_SKILL_DIR"
        echo "✅ Removed $USER_SKILL_DIR"
        echo "   Backup saved at: $backup_dir"
    else
        echo "⏭️  Keeping Claude Code skill"
    fi
else
    echo "ℹ️  Claude Code skill not found (user-level)"
fi

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Uninstallation complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
echo "Notes:"
echo "  • Local .env files are NOT removed automatically"
echo "  • Project-level skill (.claude/skills/) is NOT removed"
echo
