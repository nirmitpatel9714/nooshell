#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
INSTALL_DIR="${HOME}/.noo/bin"
BIN_NAME="noo"
RELEASE=true

echo "=== nooshell Installer ==="
echo ""

# --- Check Rust toolchain ---
if ! command -v cargo &>/dev/null; then
    echo "Error: Rust/Cargo not found. Install from https://rustup.rs and try again." >&2
    exit 1
fi

# --- Build ---
echo "Building nooshell (release)..."
(
    cd "$REPO_ROOT"
    cargo build --release
)

# On Windows (Git Bash / MSYS2 / Cygwin), the binary has .exe extension
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || -n "$MSYSTEM" ]]; then
    BIN_NAME="noo.exe"
fi

SOURCE_BIN="${REPO_ROOT}/target/release/${BIN_NAME}"

if [ ! -f "$SOURCE_BIN" ]; then
    echo "Error: Binary not found at $SOURCE_BIN" >&2
    exit 1
fi

# --- Install directory ---
echo "Installing to ${INSTALL_DIR} ..."
mkdir -p "$INSTALL_DIR"
cp "$SOURCE_BIN" "${INSTALL_DIR}/${BIN_NAME}"
chmod +x "${INSTALL_DIR}/${BIN_NAME}"
echo "Installed ${BIN_NAME} to ${INSTALL_DIR}"

# --- PATH ---
SHELL_CONFIG=""

if [ -n "${BASH-}" ]; then
    # Detect the right config file
    if [ -f "$HOME/.bashrc" ]; then
        SHELL_CONFIG="$HOME/.bashrc"
    elif [ -f "$HOME/.bash_profile" ]; then
        SHELL_CONFIG="$HOME/.bash_profile"
    fi
elif [ -n "${ZSH_VERSION-}" ]; then
    SHELL_CONFIG="$HOME/.zshrc"
fi

if [ -n "$SHELL_CONFIG" ]; then
    PATH_LINE="export PATH=\"\$HOME/.noo/bin:\$PATH\""
    if ! grep -qF '$HOME/.noo/bin' "$SHELL_CONFIG" 2>/dev/null; then
        echo "" >> "$SHELL_CONFIG"
        echo "# Added by nooshell installer" >> "$SHELL_CONFIG"
        echo "$PATH_LINE" >> "$SHELL_CONFIG"
        echo "Added ~/.noo/bin to PATH in $SHELL_CONFIG"
    else
        echo "PATH entry already exists in $SHELL_CONFIG"
    fi
else
    echo ""
    echo "NOTE: Add ~/.noo/bin to your PATH manually."
    echo "  e.g. add this to your shell config:"
    echo "    export PATH=\"\$HOME/.noo/bin:\$PATH\""
fi

echo ""
echo "=== Done ==="
echo "Restart your terminal or run: source $SHELL_CONFIG"
echo "Then run: noo --help"
