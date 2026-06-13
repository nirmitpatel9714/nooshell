#!/usr/bin/env sh
set -e

BIN_DIR="${HOME}/.noo/bin"
BINARY="${BIN_DIR}/noo"

echo "==> Building noobook (release)..."
cargo build --release --manifest-path "$(dirname "$0")/../../Cargo.toml"

echo "==> Installing binary to ${BIN_DIR}"
mkdir -p "${BIN_DIR}"
cp "$(dirname "$0")/../../target/release/noo" "${BINARY}"
chmod +x "${BINARY}"

case "${SHELL:-}" in
  *zsh*) RC_FILE="${ZDOTDIR:-$HOME}/.zshrc" ;;
  *bash*) RC_FILE="${HOME}/.bashrc" ;;
  *) RC_FILE="${HOME}/.profile" ;;
esac

case ":${PATH}:" in
  *:"${BIN_DIR}":*) ;;
  *)
    printf '\nexport PATH="%s:$PATH"\n' "${BIN_DIR}" >> "${RC_FILE}"
    echo "==> Added ${BIN_DIR} to PATH in ${RC_FILE}"
    echo "==> Restart your shell or run: export PATH=\"${BIN_DIR}:\$PATH\""
    ;;
esac

echo "==> noo installed successfully"
