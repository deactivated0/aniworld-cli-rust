<div align="center">

# 🎬 aniworld-cli-rust

**Blazing-fast, native Rust terminal client for AniWorld anime streaming with mpv integration.**

[![Rust CI](https://github.com/deactivated0/aniworld-cli-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/deactivated0/aniworld-cli-rust/actions/workflows/ci.yml)
[![Rust Version](https://img.shields.io/badge/rust-1.100%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)]()

[Features](#-key-features) • [Installation](#-installation) • [Usage](#-usage-guide) • [Configuration](#-configuration) • [MPV Integration](#-mpv-integration--shaders) • [Architecture](#-architecture--design) • [License](#-license)

</div>

---

## ⚡ Overview

**`aniworld-cli-rust`** is an independent, native Rust CLI designed for discovering, tracking, and streaming anime directly from the terminal. 

Unlike heavy script-based alternatives or bloated electron wrappers, `aniworld-cli-rust` compiles down to a single compact native binary. It streams directly via **[mpv](https://mpv.io/)** without downloading media files, saving system resources and maximizing responsiveness.

### 🌟 Key Features

- **🚀 Native & Ultra-Fast:** Built with Rust 2024 edition for instant startup and negligible memory footprint.
- **🔍 Interactive Fuzzy Search:** Fast terminal menu powered by `dialoguer` to browse series, movies, seasons, and episodes.
- **🎧 Multi-Language Support:** Instant support for German Dub (`ger-dub`), German Sub (`ger-sub`), and English Sub (`eng-sub`) with customizable language fallback.
- **🔄 Smart Host Selection & Fallback:** Automatically switches between supported providers (VOE, Vidmoly) with stream validation.
- **📺 Seamless MPV Playback:** Direct process execution without shell interpolation for maximum security; supports custom MPV flags, shaders (e.g. Anime4K), and configs.
- **🔁 Auto-Play & Binge Mode:** Continuous playback across episodes in the current season with `--auto`.
- **📑 Watch History & Watchlist:** Resume where you left off with `-w` / `--watchlist` (fully compatible with standard `state.json` formats).
- **🤖 Scriptable & JSON Output:** `--json` and `--search-json` output flags for integration into scripts, rofi, dmenu, or custom TUIs.
- **🩺 Diagnostic Doctor:** Built-in `--doctor` flag to diagnose player availability, network connectivity, and state paths.

---

## 📦 Requirements

- **Rust toolchain:** `stable` (Rust 1.100.0+ recommended)
- **Video Player:** **[mpv](https://mpv.io/)** installed and accessible via `PATH` (or specified via `--player`)
- **Operating System:** Linux, macOS, or Windows

---

## 🚀 Installation

### Option 1: Install via Cargo (Recommended)

```bash
cargo install --git https://github.com/deactivated0/aniworld-cli-rust.git
```

This installs the binary as `aniworld` directly into `~/.cargo/bin`.

---

### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/deactivated0/aniworld-cli-rust.git
cd aniworld-cli-rust

# Build optimized release binary
cargo build --release

# Optional: Install to Cargo bin directory
cargo install --path .
```

The compiled binary will be located at `target/release/aniworld` (or `target/release/aniworld.exe` on Windows).

---

### Option 3: Nix Development Shell

A reproducible `flake.nix` is provided for Nix users:

```bash
nix develop
cargo build --release
```

---

## 🎮 Usage Guide

### Basic Commands

```bash
# Interactive search and playback
aniworld "frieren"

# Binge the current season automatically
aniworld "frieren" --auto

# Resume the most recently watched series from your watchlist
aniworld -w --no-menu

# Open interactive watchlist menu
aniworld -w

# Direct episode selection (Season 1, Episode 3)
aniworld "naruto" --episode 1-3 --no-menu

# Direct movie selection (Season 0, Movie 1)
aniworld "naruto" --episode 0-1 --no-menu
```

### Language & Provider Preferences

```bash
# Prioritize German subtitles, then English subtitles
aniworld "chainsaw man" --lang ger-sub,eng-sub

# Prioritize Vidmoly hoster over VOE
aniworld "chainsaw man" --hoster Vidmoly,VOE
```

### Automation & Scripting

```bash
# Output search results in JSON format
aniworld "one piece" --search-json

# Resolve stream URL and print JSON without starting playback
aniworld "one piece" --no-menu --episode 1-1 --json

# Resolve stream with verbose debug output
aniworld "one piece" --no-menu --episode 1-1 --debug
```

### Diagnostics & History

```bash
# Show watched series history
aniworld --history

# Check mpv status, state storage, and site endpoint
aniworld --doctor
```

---

## 🛠 Command-Line Reference

| Flag / Option | Description | Default |
|---|---|---|
| `[QUERY]` | Anime title to search for | Optional |
| `-w`, `--watchlist` | Select a title from saved watch history | `false` |
| `--auto` | Automatically play next episodes in season consecutively | `false` |
| `--no-menu` | Non-interactive mode; auto-select first matching entry | `false` |
| `--episode <S-E>` | Direct episode selector (e.g., `1-5` or `0-1` for movies) | None |
| `--lang <LANGS>` | Comma-separated language priority list | `ger-dub,ger-sub,eng-sub` |
| `--hoster <HOSTS>` | Comma-separated hoster priority list | `VOE,Vidmoly` |
| `--player <PATH>` | Custom executable path or command for `mpv` | `mpv` on `PATH` |
| `--mpv-arg <ARG>` | Pass additional argument directly to mpv (repeatable) | None |
| `--debug` | Resolve and print stream information without launching mpv | `false` |
| `--json` | Print resolved stream details in structured JSON format | `false` |
| `--search-json` | Output search results as JSON | `false` |
| `--no-verify` | Skip HLS media segment probe (faster startup) | `false` |
| `--history` | Display saved watch history | `false` |
| `--doctor` | Verify player executable, state file, and endpoint reachability | `false` |
| `--ui-lang <LANG>` | Interface prompt language (`en` or `de`) | `en` |
| `--base-url <URL>` | Base URL override for AniWorld | `https://aniworld.to` |

---

## ⚙ Configuration & Environment Variables

You can configure persistent defaults through environment variables:

| Variable | Description | Example |
|---|---|---|
| `ANIWORLD_LANG` | Default language priorities | `ger-sub,eng-sub,ger-dub` |
| `ANIWORLD_HOSTER` | Default hoster priorities | `VOE,Vidmoly` |
| `ANIWORLD_STATE_DIR` | Custom directory containing `state.json` | `/path/to/custom/dir` |
| `ANIWORLD_BASE_URL` | Custom site mirror or endpoint | `https://aniworld.to` |
| `ANIWORLD_UA` | Custom HTTP & MPV User-Agent | `"Mozilla/5.0 ..."` |

### State Location

Watch progress is stored locally in `state.json`:
- **Linux/macOS:** `~/.config/aniworld-cli/state.json` (or `$XDG_CONFIG_HOME/aniworld-cli/state.json`)
- **Windows:** `%APPDATA%\aniworld-cli\state.json`

---

## 🎨 MPV Integration & Shaders

`aniworld-cli-rust` executes `mpv` directly via vector arguments without shell interpolation (`sh -c`), providing robust security against injection attacks.

### Using Shaders (e.g. Anime4K)

Pass custom mpv arguments seamlessly using `--mpv-arg`:

```bash
aniworld "frieren" --mpv-arg="--glsl-shaders=~~/shaders/Anime4K_Upscale_Denoise_CNN_x2_M.glsl"
```

You can supply `--mpv-arg` multiple times:

```bash
aniworld "frieren" \
  --mpv-arg="--fs" \
  --mpv-arg="--volume=80" \
  --mpv-arg="--force-window=immediate"
```

---

## 🏗 Architecture & Design

- **Safe Process Invocation:** Streams are handed directly to `mpv` via system `exec` without shell wrappers.
- **Privacy & Security First:** No credentials, cookies, tokens, or personal identifiers are stored or uploaded.
- **Stream Verification:** Probes playlist segments before launching playback to avoid dead links.
- **Zero Runtime Dependencies:** Pure Rust binary linked against native/rustls networking components.

---

## 🧪 Testing & Quality

To run tests and lint checks locally:

```bash
# Run unit tests
cargo test --all-targets

# Check code formatting
cargo fmt --all -- --check

# Run linter
cargo clippy --all-targets -- -D warnings
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

## ⚠️ Disclaimer

This tool is an independent client that interfaces with publicly accessible web endpoints and launches your local video player. It does not host, download, or distribute copyrighted video content. Please adhere to your local laws and the terms of service of any third-party services.
