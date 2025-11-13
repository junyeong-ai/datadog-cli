#!/usr/bin/env bash
set -e

BINARY_NAME="dd"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

echo "🗑️  Uninstalling Datadog CLI..."
echo

if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
    rm "$INSTALL_DIR/$BINARY_NAME"
    echo "✅ Removed $INSTALL_DIR/$BINARY_NAME"
else
    echo "⚠️  Binary not found at $INSTALL_DIR/$BINARY_NAME"
fi

# Remove global config (optional)
echo
read -p "Remove global configuration? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    if [ -d "$HOME/.config/datadog-cli" ]; then
        rm -rf "$HOME/.config/datadog-cli"
        echo "✅ Removed ~/.config/datadog-cli"
    else
        echo "⚠️  Global config not found"
    fi
fi

echo
echo "✅ Uninstallation complete!"
echo
echo "Note: Local .env files are NOT removed automatically."
echo "If you have DD_* variables in .env, remove them manually:"
echo "  - .env may be used by other tools"
echo "  - Only remove DD_API_KEY, DD_APP_KEY, DD_SITE if not needed"
