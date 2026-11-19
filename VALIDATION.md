# Validation record

Created: 2026-11-19

## What was checked

- Rust stable version verified against the official Rust release announcement: 1.100.0 (2026-11-12), as of this package date
- Rust 2024 remains the latest stable language edition
- Legacy watch-state schema, public CLI flags and site response structure reviewed
- Rust modules and tests written with no imported Python source or original test fixtures
- MIT application licensing, permissive dependency policy and provenance limitations documented in LICENSE-AUDIT.md
- Default toolchain tracks the latest stable Rust; CI also exercises bleeding-edge nightly
- ZIP contents and metadata can be validated using standard tools

## Not verified in this execution environment

- `cargo check`, `cargo test`, `cargo clippy`, or `cargo build`: compiler unavailable
- `cargo deny check licenses`: no Cargo and no registry access
- Requests to AniWorld and live hosting CDNs: network access unavailable
- Runtime playback via mpv: cannot verify in environment
- Linux / macOS / Windows portability: untested, CI definition is included
- No performance claims are established by benchmarks

## Required before calling the software production-ready

1. Run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` on the latest stable Rust.
2. Run `cargo test --all-targets` and fix any test failures.
3. Run a live search, one movie, one episode per language and VOE/Vidmoly hoster, and segment probing.
4. Confirm mpv playback, history writes, resume behavior, and Windows/Nix development environments.
5. Verify transitive dependencies using `cargo deny check licenses`, audit advisories and commit `Cargo.lock` before publishing.

This document discloses the difference between completed source packaging and a verified binary release.
