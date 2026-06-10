#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
RELEASE=false
TARGET="debug"
BINARY="$PROJECT_ROOT/target/$TARGET/noo"
ARGS=()

for arg in "$@"; do
  if [ "$arg" = "--release" ]; then
    RELEASE=true
    TARGET="release"
    BINARY="$PROJECT_ROOT/target/$TARGET/noo"
  else
    ARGS+=("$arg")
  fi
done

echo -e "\033[36m━━ Compiling nooshell ━━━━\033[0m"
if [ "$RELEASE" = true ]; then
  cargo build --release
else
  cargo build
fi

if [ $? -ne 0 ]; then
  echo -e "\033[31mBuild failed.\033[0m"
  exit 1
fi

echo -e "\033[32mBuild successful.\033[0m"
echo ""
echo -e "\033[90mArguments: ${ARGS[*]}\033[0m"
read -rp "Run noo? (Y/n) " choice
if [ "$choice" = "" ] || [ "$choice" = "y" ] || [ "$choice" = "Y" ]; then
  clear
  exec "$BINARY" "${ARGS[@]}"
fi
