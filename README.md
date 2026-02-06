# cldbar

Windows system tray app for monitoring AI coding tool usage. Tracks token consumption, active sessions, and daily trends for Claude, Gemini, and z.ai.

[한국어](README.ko.md)

![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue) ![React 19](https://img.shields.io/badge/React-19-61dafb) ![Rust](https://img.shields.io/badge/Rust-2021-orange)

## Features

- **Multi-provider support** — Claude Code, Gemini CLI, z.ai
- **Dual source types** — Monitor local account usage or Claude API usage (Admin API key)
- **System tray** — Lives in the Windows tray area; click to toggle the popup
- **Live stats** — Input/output tokens, active sessions, message counts (auto-refreshes every 5s)
- **7-day trend chart** — Daily usage sparkline per profile
- **API cost tracking** — Real billing data for Claude API profiles
- **Light / Dark / System theme** — Glassmorphism UI with backdrop blur
- **Multi-profile** — Add, remove, and switch between multiple provider profiles

## Tech Stack

| Layer | Tech |
|-------|------|
| Desktop runtime | Tauri v2 |
| Backend | Rust (reqwest, rusqlite, chrono, serde) |
| Frontend | React 19 + TypeScript |
| Styling | Tailwind CSS v4 + Framer Motion |
| Charts | Recharts |

## Project Structure

```
src/                        # React frontend
  components/tray/          # UI components (TrayPopup, StatCards, ...)
  hooks/                    # Data fetching hooks
  lib/                      # Types, colors, formatting, theme
  styles/                   # Global CSS with theme variables

src-tauri/src/              # Rust backend
  providers/                # Provider implementations
    claude.rs               # Claude Code (local ~/.claude)
    claude_api.rs           # Claude Admin API
    gemini.rs               # Gemini CLI (local ~/.gemini)
    zai.rs                  # z.ai (local %APPDATA%/zai)
    mod.rs                  # Provider trait
  commands.rs               # Tauri IPC commands
  profile.rs                # Config persistence
  lib.rs                    # App setup & tray logic
```

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/)
- [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) (WebView2 on Windows)

### Development

```bash
npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

Build outputs:

| File | Path |
|------|------|
| EXE (standalone) | `src-tauri/target/release/cldbar.exe` |
| NSIS installer | `src-tauri/target/release/bundle/nsis/cldbar_*-setup.exe` |
| MSI installer | `src-tauri/target/release/bundle/msi/cldbar_*.msi` |

## Configuration

Config is stored at `%APPDATA%/cldbar/config.json`. On first launch, installed providers are auto-detected:

- `~/.claude/` → Claude
- `~/.gemini/` → Gemini
- `%APPDATA%/zai/` → z.ai

Additional profiles (including Claude API) can be added from the Settings panel.

## License

MIT
