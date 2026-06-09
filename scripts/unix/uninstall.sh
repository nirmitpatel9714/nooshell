#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="${HOME}/.noo/bin"

echo "=== nooshell Uninstaller ==="
echo ""

# Remove binary
if [ -f "${INSTALL_DIR}/noo" ]; then
    rm "${INSTALL_DIR}/noo"
    echo "Removed ${INSTALL_DIR}/noo"
fi

if [ -f "${INSTALL_DIR}/noo.exe" ]; then
    rm "${INSTALL_DIR}/noo.exe"
    echo "Removed ${INSTALL_DIR}/noo.exe"
fi

# Remove install dir if empty
if [ -d "$INSTALL_DIR" ]; then
    rmdir "$INSTALL_DIR" 2>/dev/null && echo "Removed empty directory $INSTALL_DIR"
    rmdir "$(dirname "$INSTALL_DIR")" 2>/dev/null || true
fi

# Remove from shell configs
for config in "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "$HOME/.profile"; do
    if [ -f "$config" ]; then
        if grep -qF '$HOME/.noo/bin' "$config" 2>/dev/null; then
            # Remove the PATH line and the comment line above it
            sed -i '/# Added by nooshell installer/d; /\$HOME\/\.noo\/bin/d' "$config"
            echo "Removed PATH entry from $config"
        fi
    fi
done

echo ""
echo "=== Done ==="
echo "noo has been uninstalled."
