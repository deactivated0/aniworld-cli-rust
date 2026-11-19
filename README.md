<div align="center">

# 🎬 aniworld-cli-rust

**Blazing-fast, native Rust terminal client for AniWorld anime streaming with mpv integration.**

[![Rust CI](https://github.com/deactivated0/aniworld-cli-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/deactivated0/aniworld-cli-rust/actions/workflows/ci.yml)
[![Rust Version](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)]()

[Features](#-key-features) • [Installation](#-installation) • [Releases](#-pre-built-releases) • [Usage](#-usage-guide) • [Configuration](#-configuration) • [MPV Integration](#-mpv-integration--shaders) • [Architecture](#-architecture--design) • [License & Independence](#-mit-license--independence-why-it-is-not-gpl) • [AI Maintenance](#-ai-agent-maintenance-skillmd) • [License](#-license)

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

- **Rust toolchain:** `stable` (Rust 1.85.0+ / Rust Edition 2024) if building from source
- **Video Player:** **[mpv](https://mpv.io/)** installed and accessible via `PATH` (or configured via `--player`)
- **Operating Systems Supported:** Linux (all distros), macOS, Windows, FreeBSD, NixOS

---

## 📥 Pre-Built Releases

Download precompiled standalone binaries directly from the **[GitHub Releases](https://github.com/deactivated0/aniworld-cli-rust/releases)** page:

| Platform | Target Architecture | Archive |
|---|---|---|
| **Linux** | `x86_64-unknown-linux-gnu` | [Download `.tar.gz`](https://github.com/deactivated0/aniworld-cli-rust/releases/latest) |
| **macOS (Apple Silicon)** | `aarch64-apple-darwin` | [Download `.tar.gz`](https://github.com/deactivated0/aniworld-cli-rust/releases/latest) |
| **macOS (Intel)** | `x86_64-apple-darwin` | [Download `.tar.gz`](https://github.com/deactivated0/aniworld-cli-rust/releases/latest) |
| **Windows** | `x86_64-pc-windows-msvc` | [Download `.zip`](https://github.com/deactivated0/aniworld-cli-rust/releases/latest) |

### Quick Install (Linux & macOS)

```bash
# Download and install the latest binary to /usr/local/bin
curl -sSL https://raw.githubusercontent.com/deactivated0/aniworld-cli-rust/main/scripts/install.sh | bash
```
*(Or extract the release archive and move `aniworld` into any directory on your `$PATH`)*

---

## 🚀 Installation by Platform

### 🐧 Linux (All Major Distributions)

Before installing, ensure **`mpv`** is installed on your distribution:

<details open>
<summary><b>1. Arch Linux / Manjaro / EndeavourOS</b></summary>

```bash
# Install mpv and Rust toolchain
sudo pacman -S mpv rustup
rustup default stable

# Install aniworld-cli-rust via cargo
cargo install --git https://github.com/deactivated0/aniworld-cli-rust.git
```
*(AUR package `aniworld-cli-rust-bin` / `aniworld-cli-rust-git` can also be packaged using `makepkg -si`)*
</details>

<details>
<summary><b>2. Ubuntu / Debian / Pop!_OS / Linux Mint</b></summary>

```bash
# Install mpv and build dependencies
sudo apt update
sudo apt install -y mpv curl build-essential pkg-config libssl-dev

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install aniworld-cli-rust
cargo install --git https://github.com/deactivated0/aniworld-cli-rust.git
```
</details>

<details>
<summary><b>3. Fedora / RHEL / CentOS Stream</b></summary>

```bash
# Install mpv (RPM Fusion) and tools
sudo dnf install -y mpv gcc pkgconf-pkg-config openssl-devel

# Install via Cargo
cargo install --git https://github.com/deactivated0/aniworld-cli-rust.git
```
</details>

<details>
<summary><b>4. openSUSE (Tumbleweed / Leap)</b></summary>

```bash
sudo zypper install -y mpv rust cargo gcc pkg-config libopenssl-devel
cargo install --git https://github.com/deactivated0/aniworld-cli-rust.git
```
</details>

<details>
<summary><b>5. Alpine Linux</b></summary>

```bash
sudo apk add mpv rust cargo build-base pkgconf openssl-dev
cargo install --git https://github.com/deactivated0/aniworld-cli-rust.git
```
</details>

<details>
<summary><b>6. Void Linux</b></summary>

```bash
sudo xbps-install -S mpv rust cargo base-devel pkg-config openssl-devel
cargo install --git https://github.com/deactivated0/aniworld-cli-rust.git
```
</details>

<details>
<summary><b>7. Gentoo Linux</b></summary>

```bash
sudo emerge --ask media-video/mpv dev-lang/rust
cargo install --git https://github.com/deactivated0/aniworld-cli-rust.git
```
</details>

---

### ❄️ NixOS & Nix (With Flakes)

`aniworld-cli-rust` provides native Nix Flake support out of the box with zero compilation configuration needed.

#### Run directly without installing:

```bash
nix run github:deactivated0/aniworld-cli-rust -- "frieren"
```

#### Install to user profile:

```bash
nix profile install github:deactivated0/aniworld-cli-rust
```

#### Add to NixOS Configuration (`flake.nix`):

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    aniworld-cli.url = "github:deactivated0/aniworld-cli-rust";
  };

  outputs = { self, nixpkgs, aniworld-cli, ... }: {
    nixosConfigurations.myhostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ({ pkgs, ... }: {
          environment.systemPackages = [
            pkgs.mpv
            aniworld-cli.packages.${pkgs.system}.default
          ];
        })
      ];
    };
  };
}
```

> [!TIP]
> **Important Note on `follows = "nixpkgs"`:**
> Avoid adding `inputs.aniworld-cli.inputs.nixpkgs.follows = "nixpkgs"` unless your system's `nixpkgs` channel includes **Rust 1.85+** (Rust 2024 edition). 
> Leaving `follows` omitted allows `aniworld-cli` to use its own pinned `nixos-unstable` nixpkgs input with a modern Rust toolchain.

#### Or use the Nix Overlay:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    aniworld-cli.url = "github:deactivated0/aniworld-cli-rust";
  };

  outputs = { self, nixpkgs, aniworld-cli, ... }: {
    nixosConfigurations.myhostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        {
          nixpkgs.overlays = [ aniworld-cli.overlays.default ];
          environment.systemPackages = with pkgs; [
            mpv
            aniworld-cli
          ];
        }
      ];
    };
  };
}
```

#### Troubleshooting Nix Builds:

> [!WARNING]
> **Error:** `error: rustc 1.xx is not supported by the following package: aniworld-cli-rust requires rustc 1.85`
>
> * **Cause:** Your system `flake.nix` has `aniworld-cli.inputs.nixpkgs.follows = "nixpkgs"` pointing to a NixOS channel whose Rust compiler is older than Rust 1.85 (the minimum version for Rust 2024 edition).
> * **Solution 1:** In `/etc/nixos/flake.nix`, remove the `aniworld-cli.inputs.nixpkgs.follows = "nixpkgs";` line so Nix uses `aniworld-cli`'s up-to-date toolchain. Then run:
>   ```bash
>   nix flake update aniworld-cli
>   sudo nixos-rebuild switch --flake /etc/nixos#myhostname
>   ```
> * **Solution 2:** Update your host `nixpkgs` input to the latest `nixos-unstable` channel:
>   ```bash
>   nix flake lock --update-input nixpkgs
>   ```
> * **Solution 3:** Alternatively, run `nix run github:deactivated0/aniworld-cli-rust` or install a pre-compiled binary from [GitHub Releases](https://github.com/deactivated0/aniworld-cli-rust/releases).

#### Or use the Nix development shell:

```bash
git clone https://github.com/deactivated0/aniworld-cli-rust.git
cd aniworld-cli-rust
nix develop
cargo build --release
```

---

### 🍎 macOS

1. Install **[Homebrew](https://brew.sh/)** and **`mpv`**:
   ```bash
   brew install mpv rustup
   rustup-init -y
   ```

2. Install `aniworld-cli-rust`:
   ```bash
   cargo install --git https://github.com/deactivated0/aniworld-cli-rust.git
   ```

*(Alternatively, grab the precompiled macOS Apple Silicon or Intel binary from [GitHub Releases](https://github.com/deactivated0/aniworld-cli-rust/releases).)*

---

### 🪟 Windows

1. Install **`mpv`** using your favorite package manager:
   ```powershell
   # Via winget
   winget install mpv.net
   # Or via scoop
   scoop install mpv
   ```

2. Install `aniworld-cli-rust`:
   ```powershell
   # Option A: Download pre-built zip from GitHub Releases
   # Unzip aniworld.exe and place it in your PATH (e.g. C:\Windows\System32 or ~/bin)

   # Option B: Install via Cargo
   cargo install --git https://github.com/deactivated0/aniworld-cli-rust.git
   ```

*`aniworld-cli-rust` automatically detects `mpv.exe` in your `%PATH%` or bundled adjacent to the executable.*

---

### 😈 FreeBSD

1. Install prerequisites:
   ```sh
   pkg install mpv rust
   ```

2. Build and install:
   ```sh
   cargo install --git https://github.com/deactivated0/aniworld-cli-rust.git
   ```

---

### 🔨 Build from Git Source

```bash
# Clone the repository
git clone https://github.com/deactivated0/aniworld-cli-rust.git
cd aniworld-cli-rust

# Build release binary
cargo build --release

# Install locally
cargo install --path .
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

### 📺 Rich Interactive Post-Playback Menu

When an episode finishes or when closing the player, `aniworld-cli-rust` presents a categorized, responsive TUI menu with keyboard navigation (Arrow Up/Down, PageUp/PageDown, Home/End, or Vim `j`/`k`):

```text
Resolved via VOE (ger-dub)

? After playback ›
  ── CONTINUE ──────────────────────
❯ Next episode
  Previous episode
  Replay episode
  Resume playback
  Next unwatched episode
  Random episode

  ── NAVIGATION ────────────────────
  Select episode
  Select season
  Jump to episode
  Search another anime
  Continue another anime

  ── STREAM & LANGUAGE ─────────────
  Change language
  Change hoster
  Change stream quality
  Find fastest hoster
  Retry stream
  Test all hosters

  ── PLAYBACK ──────────────────────
  Change playback speed
  Select audio track
  Select subtitle track
  Toggle auto-next
  Toggle skip intro
  Toggle skip outro
  Player settings

  ── VIDEO ENHANCEMENTS ────────────
  Toggle Anime4K
  Select Anime4K preset
  Video filters
  Shader settings

  ── LIBRARY ───────────────────────
  Open watchlist
  Open favorites
  View watch history
  Add/remove favorite
  Mark as watched
  Mark as unwatched

  ── TOOLS ─────────────────────────
  Episode information
  Playback statistics
  Copy episode URL
  Copy stream URL
  Open in browser
  Refresh episode list
  Refresh stream sources
  Clear cache

  ── SYSTEM ────────────────────────
  Settings
  Quit
```

- **Smooth Scrolling & Viewport Clamping:** The menu automatically adapts to your terminal height, scrolling categories smoothly without overflowing or corrupting terminal scrollback buffers.
- **Section Skipping:** Arrow keys automatically navigate between active action items and skip over category header dividers.

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

## ⚖️ MIT License & Independence (Why It Is NOT GPL)

`aniworld-cli-rust` is released under the permissive **[MIT License](LICENSE)** and is an entirely separate, clean-room implementation from other projects in the anime streaming CLI space.

### Key Distinctions from Other Projects

| Feature / Aspect | `aniworld-cli-rust` (This Project) | [`dxmoc/aniworld-cli`](https://github.com/dxmoc/aniworld-cli/) | [`pystardust/ani-cli`](https://github.com/pystardust/ani-cli) |
|---|---|---|---|
| **License** | **MIT (Permissive)** | GPL-3.0 (Copyleft) | GPL-3.0 (Copyleft) |
| **Language** | **Pure Native Rust (2024 Edition)** | Python 3 | POSIX Bash Shell Script |
| **Runtime Requirements** | **None** (Single static/native binary) | Python 3, `pip`, `requests`, `bs4`, etc. | `bash`, `curl`, `grep`, `sed`, `fzf`, etc. |
| **Target Platform / Scope** | **AniWorld** (German Dub, German Sub, English Sub) | AniWorld | Scrapes English sites (AllAnime, Gogoanime) |
| **Code Sharing / Lineage** | **100% original Rust code**; zero imported source | Original Python codebase | Shell script codebase |
| **Security Architecture** | Direct OS vector execution (`std::process::Command`), zero shell injection | Python subprocess calls | Shell piping & variable interpolation |
| **Stream Probing** | Built-in concurrent HLS playlist validation | Basic HTTP check | Shell curl wrapper |
| **Dependency Auditing** | Enforced by `cargo-deny` in CI (GPL/AGPL strictly rejected) | None | N/A |

### Why MIT and Not GPL?

1. **Permissive Freedom Without Viral Restrictions:**
   The MIT license gives users and developers maximum freedom to integrate, embed, package, or distribute the application without being subject to the copyleft requirements of GPL-3.0.

2. **Zero Code Sharing:**
   `aniworld-cli-rust` does not fork, vendor, translate, or copy code from `dxmoc/aniworld-cli` or `pystardust/ani-cli`. All parsers, host extractors (VOE, Vidmoly), CLI options, and network layers were authored from the ground up in idiomatic Rust.

3. **Interoperability Without Infiltration:**
   The tool is deliberately schema-compatible with the standard `state.json` watch-history format so users can migrate painlessly, but the underlying state engine and logic are implemented using standard `serde`/`serde_json` in Rust with independent serialization.

4. **Strict Permissive Dependency Enforcement:**
   Every single dependency in `Cargo.lock` is audited using [`cargo-deny`](deny.toml) in our continuous integration pipeline. Any crate introducing GPL or AGPL copyleft terms is rejected at compile time.

---

## 🤖 AI Agent Maintenance (SKILL.md)

This repository includes a dedicated **[`SKILL.md`](SKILL.md)** file designed to guide AI coding assistants (such as Antigravity, Claude Code, Cursor, Copilot, Codex, Aider) in maintaining, extending, and debugging the project:

- **Architecture Map:** Detailed overview of modules (`main`, `catalog`, `resolve`, `net`, `player`, `options`, `state`).
- **Standard Workflows:** Step-by-step recipes for adding streaming providers, updating HTML selectors, and auditing dependencies.
- **Strict Invariants:** Enforces MIT-only dependency rules, zero shell-injection policy, and `state.json` backward compatibility.
- **Quality Gates:** Ready-to-run verification scripts matching CI requirements.

Consult [SKILL.md](SKILL.md) whenever prompting an AI assistant to modify this codebase.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

## ⚠️ Disclaimer

This tool is an independent client that interfaces with publicly accessible web endpoints and launches your local video player. It does not host, download, or distribute copyrighted video content. Please adhere to your local laws and the terms of service of any third-party services.
