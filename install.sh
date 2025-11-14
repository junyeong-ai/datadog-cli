#!/usr/bin/env bash
set -e

BINARY_NAME="datadog"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
SKILL_NAME="datadog-query"
PROJECT_SKILL_DIR=".claude/skills/$SKILL_NAME"
USER_SKILL_DIR="$HOME/.claude/skills/$SKILL_NAME"

echo "🚀 Installing Datadog CLI..."
echo

# ============================================================================
# Skill Installation Functions
# ============================================================================

get_skill_version() {
    local skill_md="$1"
    if [ -f "$skill_md" ]; then
        grep "^version:" "$skill_md" 2>/dev/null | sed 's/version: *//' || echo "unknown"
    else
        echo "unknown"
    fi
}

check_skill_exists() {
    [ -d "$USER_SKILL_DIR" ] && [ -f "$USER_SKILL_DIR/SKILL.md" ]
}

compare_versions() {
    local ver1="$1"
    local ver2="$2"

    if [ "$ver1" = "$ver2" ]; then
        echo "equal"
    elif [ "$ver1" = "unknown" ] || [ "$ver2" = "unknown" ]; then
        echo "unknown"
    else
        if [ "$(printf '%s\n' "$ver1" "$ver2" | sort -V | head -n1)" = "$ver1" ]; then
            if [ "$ver1" != "$ver2" ]; then
                echo "older"
            else
                echo "equal"
            fi
        else
            echo "newer"
        fi
    fi
}

backup_skill() {
    local timestamp=$(date +%Y%m%d-%H%M%S)
    local backup_dir="$USER_SKILL_DIR.bak-$timestamp"

    echo "📦 Creating backup: $backup_dir"
    cp -r "$USER_SKILL_DIR" "$backup_dir"
    echo "   ✅ Backup created successfully"
}

install_user_level_skill() {
    echo "📋 Installing skill to $USER_SKILL_DIR"

    mkdir -p "$(dirname "$USER_SKILL_DIR")"
    cp -r "$PROJECT_SKILL_DIR" "$USER_SKILL_DIR"

    echo "   ✅ User-level skill installed successfully"
}

install_project_level_skill() {
    echo "✅ Project-level skill already available at: $PROJECT_SKILL_DIR"
    echo "   This skill is project-specific and works when Claude Code is opened here."
}

prompt_skill_installation() {
    if [ ! -d "$PROJECT_SKILL_DIR" ]; then
        echo "ℹ️  No Claude Code skill found in project"
        return 0
    fi

    local project_version=$(get_skill_version "$PROJECT_SKILL_DIR/SKILL.md")

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "🤖 Claude Code Skill Installation"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "This project includes a Claude Code skill for Datadog CLI queries."
    echo "The skill enables Claude to execute Datadog queries automatically."
    echo ""
    echo "Skill: $SKILL_NAME (v$project_version)"
    echo ""

    # Check if user-level skill exists
    if check_skill_exists; then
        local existing_version=$(get_skill_version "$USER_SKILL_DIR/SKILL.md")
        local comparison=$(compare_versions "$existing_version" "$project_version")

        echo "Status: Already installed at user-level (v$existing_version)"
        echo ""

        case "$comparison" in
            equal)
                echo "✅ You have the latest version installed"
                echo ""
                read -p "Reinstall anyway? [y/N]: " choice
                case "$choice" in
                    y|Y)
                        backup_skill
                        rm -rf "$USER_SKILL_DIR"
                        install_user_level_skill
                        ;;
                    *)
                        echo "   ⏭️  Skipped"
                        ;;
                esac
                ;;
            older)
                echo "🔄 New version available: v$project_version"
                echo ""
                read -p "Update to v$project_version? [Y/n]: " choice
                case "$choice" in
                    n|N)
                        echo "   ⏭️  Keeping current version"
                        ;;
                    *)
                        backup_skill
                        rm -rf "$USER_SKILL_DIR"
                        install_user_level_skill
                        echo "   ✅ Updated to v$project_version"
                        ;;
                esac
                ;;
            newer)
                echo "⚠️  Your installed version (v$existing_version) is newer than project version (v$project_version)"
                echo ""
                read -p "Downgrade to v$project_version? [y/N]: " choice
                case "$choice" in
                    y|Y)
                        backup_skill
                        rm -rf "$USER_SKILL_DIR"
                        install_user_level_skill
                        ;;
                    *)
                        echo "   ⏭️  Keeping current version"
                        ;;
                esac
                ;;
            *)
                echo "⚠️  Version comparison failed"
                echo ""
                read -p "Reinstall anyway? [y/N]: " choice
                case "$choice" in
                    y|Y)
                        backup_skill
                        rm -rf "$USER_SKILL_DIR"
                        install_user_level_skill
                        ;;
                    *)
                        echo "   ⏭️  Skipped"
                        ;;
                esac
                ;;
        esac
    else
        # No existing user-level skill - show installation options
        echo "Installation options:"
        echo ""
        echo "  [1] Skip      - Don't install skill (you can install later)"
        echo "  [2] User      - Install to ~/.claude/skills/ (RECOMMENDED)"
        echo "  [3] Project   - Keep in ./.claude/skills/ (current project only)"
        echo "  [4] Both      - Install to both locations (user + project)"
        echo ""
        read -p "Choose installation option [1-4] (default: 2): " choice
        echo

        case "$choice" in
            1)
                echo "⏭️  Skill installation skipped"
                echo ""
                echo "To install later:"
                echo "  • User-level:    cp -r $PROJECT_SKILL_DIR ~/.claude/skills/"
                echo "  • Project-level: Already available at $PROJECT_SKILL_DIR"
                ;;
            2|"")
                install_user_level_skill
                echo ""
                echo "🎉 Skill installed successfully!"
                echo ""
                echo "Claude Code can now:"
                echo "  • Execute Datadog queries automatically"
                echo "  • Parse natural language time expressions"
                echo "  • Build monitoring dashboards"
                echo "  • Investigate production errors"
                ;;
            3)
                echo ""
                install_project_level_skill
                ;;
            4)
                install_user_level_skill
                echo ""
                install_project_level_skill
                echo ""
                echo "🎉 Skill installed at both locations!"
                ;;
            *)
                echo "❌ Invalid option. Skipping skill installation."
                echo ""
                echo "To install later, run this script again or copy manually:"
                echo "  cp -r $PROJECT_SKILL_DIR ~/.claude/skills/"
                ;;
        esac
    fi

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# ============================================================================
# Binary Installation
# ============================================================================

echo "📦 Building release binary..."
cargo build --release

# Create install directory if it doesn't exist
mkdir -p "$INSTALL_DIR"

# Copy binary
echo "📋 Installing to $INSTALL_DIR/$BINARY_NAME"
cp "target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# macOS: Ad-hoc sign the binary to prevent "Killed: 9" errors
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "🔏 Signing binary (macOS)..."
    codesign --force --deep --sign - "$INSTALL_DIR/$BINARY_NAME" 2>/dev/null || true
fi

echo
echo "✅ Binary installation complete!"
echo
echo "Binary installed to: $INSTALL_DIR/$BINARY_NAME"
echo

# Check if in PATH
if echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo "✅ $INSTALL_DIR is in your PATH"
    echo
    echo "You can now run: $BINARY_NAME --help"
else
    echo "⚠️  $INSTALL_DIR is not in your PATH"
    echo
    echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo
    echo "Then reload your shell:"
    echo "  source ~/.zshrc  # or ~/.bashrc"
fi
echo

# Check version
if command -v "$BINARY_NAME" &> /dev/null; then
    echo "Installed version:"
    "$BINARY_NAME" --version
    echo
fi

# ============================================================================
# Skill Installation Prompt
# ============================================================================

prompt_skill_installation

# ============================================================================
# Final Message
# ============================================================================

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎉 Installation Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📝 Next steps:"
echo ""
echo "1. Configure Datadog credentials:"
echo "   $BINARY_NAME config init"
echo "   $BINARY_NAME config edit"
echo ""
echo "2. Try a query:"
echo "   $BINARY_NAME monitors list"
echo "   $BINARY_NAME logs search \"status:error\" --from \"1 hour ago\""
echo ""
echo "3. Get help:"
echo "   $BINARY_NAME --help"
echo ""
echo "Documentation: https://github.com/junyeong-ai/datadog-cli"
echo ""
