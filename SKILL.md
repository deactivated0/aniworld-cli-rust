---
name: maintain-aniworld-cli-rust
description: Comprehensive maintenance, architecture, and contribution guidelines for AI agents working on aniworld-cli-rust.
---

# AI Agent Maintenance Guide: `aniworld-cli-rust`

This document provides AI agents and automated contributors with instructions, architectural mental models, invariants, and testing recipes for maintaining `aniworld-cli-rust`.

---

## 🏛 1. Architectural Map

The project is structured as a single high-performance binary crate (`aniworld`) located in `src/`:

| Module | File | Purpose & Responsibilities |
|---|---|---|
| **Entry & Dispatch** | `src/main.rs` | CLI entry point, 8-category interactive post-playback TUI (`console::Term`), paged scrolling episode selectors, auto-play loop, doctor command. |
| **Catalog & Scraping** | `src/catalog.rs` | HTML scraping via `scraper`/`html5ever`. Parses series search, seasons, episodes, languages, and host buttons. |
| **Stream Resolution** | `src/resolve.rs` | Extracts direct HLS `.m3u8` streams from providers (VOE, Vidmoly). Decodes redirects and obfuscated payloads. |
| **Network & Validation** | `src/net.rs` | `reqwest` client wrapper with custom browser User-Agent headers; concurrent HLS media-segment verification. |
| **Player Execution** | `src/player.rs` | Safe, shell-less invocation of `mpv` via `std::process::Command`. Resolves player binary across OS platforms. |
| **CLI Options** | `src/options.rs` | `clap` v4 derive definitions, environment variable fallbacks (`ANIWORLD_*`), language and hoster priority parsing. |
| **State & Watchlist** | `src/state.rs` | Schema-compatible `state.json` watch history & favorites persistence; handles platform-specific XDG / AppData config paths. |
| **Data Models** | `src/model.rs` | Core domain structs: `Series`, `Episode`, `Host`, `Stream`, `Progress`. |

---

## 🔒 2. Non-Negotiable Invariants

When modifying or generating code for this repository, AI agents **MUST** enforce the following principles:

1. **Permissive Licensing (MIT) Only:**
   - The project is strictly **MIT licensed**.
   - **NEVER** import or port code from GPL, AGPL, or copyleft repositories.
   - Any added dependency in `Cargo.toml` must be permissively licensed (MIT, Apache-2.0, BSD, ISC, CC0, etc.). It must satisfy `deny.toml`.

2. **Absolute Shell-Injection Safety:**
   - `src/player.rs` **must never** use `sh -c`, `bash -c`, `cmd.exe /c`, or raw string command execution.
   - All arguments passed to `mpv` must be passed as an isolated argument slice (`Vec<String>`).

3. **No Terminal Scrollback Overflow / Arrow Freezing (TUI Viewport Invariant):**
   - Dialoguer lists (episodes, seasons, anime search) **must always** be bounded by `.max_length(clamp(6, 12))` or a sliding viewport.
   - If a menu renders more lines than terminal height, raw ANSI escapes cannot navigate above the scrollback threshold, causing arrow key freezes.
   - The interactive post-playback menu (`run_after_playback_menu`) uses a sliding window clamped between 8 and 22 rows on `console::Term`, dynamically redrawing only the visible slice and auto-skipping category headers and blanks.

4. **No Private/Sensitive Data Exposure:**
   - Do not add features that harvest, transmit, or cache personal accounts, cookies, or credentials.
   - Example paths in tests or docs must use generic placeholders (`/home/user/...` or cross-platform relative paths).

5. **Backward Compatibility for `state.json`:**
   - Watch history format must remain backward-compatible with existing client state files.
   - All newly added fields (e.g. `favorites`) must include `#[serde(default)]` to safely parse older state versions.

---

## 🛠 3. Common AI Maintenance Tasks

### Task A: Updating or Adding a Stream Provider (Hoster)
1. Navigate to `src/resolve.rs`.
2. Implement an extraction function `fn extract_<provider>(html: &str, net: &Network, ua: &str) -> Result<Option<Stream>>`.
3. Add a unit test in `src/resolve.rs` using mock HTML/JavaScript payloads representing the provider's embed page.
4. Hook the new extractor into `extract()` inside `src/resolve.rs`.
5. Update `--hoster` documentation and defaults if applicable in `src/options.rs` and `README.md`.

### Task B: Fixing Website Layout Changes (HTML Selectors)
1. AniWorld site DOM changes will reflect in `src/catalog.rs`.
2. Inspect the selectors defined with `select(...)`.
3. Update parsing logic in `search_results()`, `seasons()`, `episodes()`, or `available_languages()`.
4. Ensure tests in `src/catalog.rs` pass with representative HTML fixtures.

### Task C: Upgrading Dependencies & License Auditing
1. Check dependencies with `cargo outdated` or edit `Cargo.toml`.
2. Update lockfile: `cargo update`.
3. Verify license compliance:
   ```bash
   cargo install cargo-deny --locked
   cargo deny check licenses
   ```
4. If a legitimate new permissive license is introduced by a dependency (e.g. a data license like `CDLA-Permissive-2.0`), add it to `deny.toml`.

### Task D: Extending the Post-Playback Interactive Menu
1. `AFTER_PLAYBACK_ROWS` in `src/main.rs` defines the visual menu structure (headers, actions, and spacers).
2. `MenuAction` enum lists every selectable action.
3. Category headers (`MenuRow::Header(...)`) and separators (`MenuRow::Blank`) are automatically skipped during keyboard navigation (`Up`, `Down`, `PageUp`, `PageDown`).
4. Handle any newly added action inside the `match action` dispatcher in `App::run_after_playback_menu`.
5. If the action persists user preference, update `src/state.rs` and verify serde compatibility.

---

## 🧪 4. Quality Gate & CI Verification

Before submitting any commit or pull request, run the exact verification pipeline used by GitHub Actions CI:

```bash
# 1. Format check
cargo fmt --all -- --check

# 2. Strict linter check (zero warnings tolerated)
cargo clippy --all-targets -- -D warnings

# 3. Unit tests
cargo test --all-targets

# 4. Release compile
cargo build --release

# 5. CLI sanity check
./target/release/aniworld --help
```

---

## 🚀 5. Release Workflow

To create a new release:
1. Bump `version = "X.Y.Z"` in `Cargo.toml` and update `flake.nix`.
2. Commit the change: `git commit -am "chore: bump version to vX.Y.Z"`.
3. Tag the commit: `git tag vX.Y.Z`.
4. Push tag to GitHub: `git push origin vX.Y.Z`.
5. GitHub Actions (`.github/workflows/release.yml`) will automatically cross-compile binaries for Linux, macOS, and Windows, generate `SHA256SUMS.txt`, and publish the release.
