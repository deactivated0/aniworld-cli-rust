# MIT release audit and provenance

Date: 19 November 2026

## Scope of the MIT grant

`LICENSE` and `Cargo.toml` declare MIT for the **original application source** in
this repository: `src/`, self-authored tests, build scripts, CI definitions and
project documentation. This repository includes no copied upstream Python files,
upstream test fixtures, images, binary assets or bundled mpv executable.

The application implements CLI behavior, AniWorld's public page formats and a
versioned JSON watch-history schema for interoperability. The format, feature
requirements and command-line conventions are not a license grant for any
upstream implementation. The application was authored in Rust and is not a
textual translation of the upstream Python files.

**Limit of this review:** A previous stage of development inspected upstream
GPL implementation details. Thus this is **not** a documented clean-room
implementation, and an engineering review alone cannot establish that every
protectable expression or other intellectual-property right is free from
third-party claims. The source is MIT *licensed*, but only a qualified legal
review or permission from rights holders can provide a stronger assurance.

## Dependencies and external applications

- Dependencies in `Cargo.toml` are separately authored open-source crates. Their
  own license notices and obligations continue to apply in distributed binaries.
- `deny.toml` provides a strict allowlist of permissive dependency licenses.
  The GitHub Actions `licenses` job executes `cargo deny check licenses` and
  fails for unmatched or unknown dependency licenses. The initial release gate
  has **not** yet run in this environment.
- mpv is *not embedded or redistributed*. It is a program independently
  installed by the user and spawned as a separate process. See mpv's own
  licensing terms if you redistribute it.
- AniWorld, its website markup and streaming providers are independent
  third-party systems; an MIT license on this client does not grant rights to
  hosted content, brands, or services.

## Release checklist

- [x] Project and source files carry MIT metadata
- [x] No bundled or imported upstream GPL source or test fixtures
- [x] Site-specific functionality expressed in Rust functions
- [x] Dependency license policy declared, rejects GPL/AGPL
- [x] Latest stable Rust tracked in `rust-toolchain.toml`
- [x] Nightly smoke-test CI added
- [x] Cargo compile, tests, clippy and formatting
- [ ] `cargo deny check licenses` (runs via CI)
- [ ] Human review of provenance and source similarity
- [x] Live website and mpv integration tests

## Manual verification commands

```sh
rustup update stable
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo install cargo-deny --locked
cargo generate-lockfile
cargo deny check licenses
cargo build --release
```

The MIT notice covers the repository's original code, not all transitive
crates, external tools, site markup or streamed media. Do not market the
package as independently legally certified or operationally verified until
review and the remaining release gates are completed.
