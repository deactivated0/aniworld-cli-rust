#!/usr/bin/env bash
set -euo pipefail

REPO="deactivated0/aniworld-cli-rust"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Fallback to ~/.local/bin if /usr/local/bin is not writable without sudo
if [ ! -w "$INSTALL_DIR" ] && [ "$(id -u)" -ne 0 ]; then
  INSTALL_DIR="$HOME/.local/bin"
  mkdir -p "$INSTALL_DIR"
fi

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
  linux)
    TARGET="x86_64-unknown-linux-gnu"
    ;;
  darwin)
    if [ "$ARCH" = "arm64" ]; then
      TARGET="aarch64-apple-darwin"
    else
      TARGET="x86_64-apple-darwin"
    fi
    ;;
  *)
    echo "Unsupported OS: $OS. Please install via cargo: cargo install --git https://github.com/$REPO.git"
    exit 1
    ;;
esac

ASSET="aniworld-cli-rust-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"

echo "Fetching ${ASSET}..."
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

if command -v curl >/dev/null 2>&1; then
  curl -sSL "$URL" -o "$TMPDIR/$ASSET"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$TMPDIR/$ASSET" "$URL"
else
  echo "Error: curl or wget is required" >&2
  exit 1
fi

tar -xzf "$TMPDIR/$ASSET" -C "$TMPDIR"
chmod +x "$TMPDIR/aniworld"
mv "$TMPDIR/aniworld" "$INSTALL_DIR/aniworld"

echo "Successfully installed aniworld to $INSTALL_DIR/aniworld"
"$INSTALL_DIR/aniworld" --help | head -n 4
