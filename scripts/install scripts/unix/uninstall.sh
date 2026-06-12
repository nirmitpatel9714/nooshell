#!/usr/bin/env sh
set -e

BIN_DIR="${HOME}/.noo/bin"
BINARY="${BIN_DIR}/noo"

if [ -f "${BINARY}" ]; then
  echo "==> Removing binary: ${BINARY}"
  rm -f "${BINARY}"
fi

if [ -d "${BIN_DIR}" ] && [ -z "$(ls -A "${BIN_DIR}" 2>/dev/null)" ]; then
  rmdir "${BIN_DIR}" 2>/dev/null || true
fi

if [ -d "${HOME}/.noo" ] && [ -z "$(ls -A "${HOME}/.noo" 2>/dev/null)" ]; then
  rmdir "${HOME}/.noo" 2>/dev/null || true
fi

for RC_FILE in "${HOME}/.zshrc" "${HOME}/.bashrc" "${HOME}/.profile"; do
  if [ -f "${RC_FILE}" ]; then
    tmp=$(mktemp)
    grep -v "export PATH=\"${BIN_DIR}:\$PATH\"" "${RC_FILE}" > "${tmp}" 2>/dev/null || true
    grep -v "export PATH=\"${BIN_DIR}:\$PATH\"" "${RC_FILE}" > "${tmp}" 2>/dev/null || true
    mv "${tmp}" "${RC_FILE}"
  fi
done

echo "==> noo uninstalled"
