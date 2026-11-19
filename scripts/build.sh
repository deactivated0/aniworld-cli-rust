#!/usr/bin/env bash
set -euo pipefail

if ! command -v rustup >/dev/null || ! command -v cargo >/dev/null; then
  printf '%s\n' 'Install rustup and Cargo from https://rustup.rs before building.' >&2
  exit 1
fi

# The default project toolchain is the latest stable Rust.
# Run `./scripts/build.sh nightly` to test bleeding-edge nightly.
channel="${1:-stable}"
if [[ "$channel" != "stable" && "$channel" != "nightly" ]]; then
  printf '%s\n' 'Usage: scripts/build.sh [stable|nightly]' >&2
  exit 2
fi

rustup toolchain install "$channel" --profile minimal --component rustfmt --component clippy
cargo "+$channel" fmt --all
cargo "+$channel" clippy --all-targets -- -D warnings
cargo "+$channel" test --all-targets
cargo "+$channel" build --release
printf 'Built with Rust %s: target/release/aniworld\n' "$channel"
