#!/usr/bin/env bash
# Builds assets/css/app.css with the Tailwind standalone CLI.
#
# The standalone CLI is a single binary with no Node.js dependency. This script
# downloads the pinned version into .tailwind/ on first use (that directory is
# git-ignored) and then runs it.
#
#   ./scripts/tailwind.sh          build once, minified
#   ./scripts/tailwind.sh watch    rebuild on every template change
set -euo pipefail

TAILWIND_VERSION="v4.3.3"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="$ROOT/.tailwind"

case "$(uname -s)" in
  Darwin) os="macos" ;;
  Linux)  os="linux" ;;
  MINGW*|MSYS*|CYGWIN*) os="windows" ;;
  *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac

case "$(uname -m)" in
  arm64|aarch64) arch="arm64" ;;
  x86_64|amd64)  arch="x64" ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

asset="tailwindcss-${os}-${arch}"
[ "$os" = "windows" ] && asset="${asset}.exe"
bin="$BIN_DIR/${asset}-${TAILWIND_VERSION}"
[ "$os" = "windows" ] && bin="${bin}.exe"

if [ ! -x "$bin" ]; then
  echo "Downloading Tailwind CLI ${TAILWIND_VERSION} (${os}-${arch})..."
  mkdir -p "$BIN_DIR"
  url="https://github.com/tailwindlabs/tailwindcss/releases/download/${TAILWIND_VERSION}/${asset}"
  curl --fail --location --progress-bar --output "$bin.tmp" "$url"
  chmod +x "$bin.tmp"
  mv "$bin.tmp" "$bin"
fi

cd "$ROOT"
if [ "${1:-build}" = "watch" ]; then
  exec "$bin" -i ./assets/css/input.css -o ./assets/css/app.css --watch
fi
exec "$bin" -i ./assets/css/input.css -o ./assets/css/app.css --minify
