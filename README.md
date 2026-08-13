# cldbar

Windows system tray app for monitoring AI coding tool usage. Shows how much of your plan is left and when it resets, alongside token consumption, active sessions, and daily trends for Claude, Codex, and Gemini.

[한국어](README.ko.md)

![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue) ![React 19](https://img.shields.io/badge/React-19-61dafb) ![Rust](https://img.shields.io/badge/Rust-2021-orange)

[![cldbar demo](https://img.youtube.com/vi/1vVVtt5Iag0/maxresdefault.jpg)](https://youtu.be/1vVVtt5Iag0)


## Features

- **Multi-provider support** — Claude Code, Codex CLI, Gemini CLI
- **Plan limits up front** — The session and weekly windows lead the popup, showing what is left and when it comes back at equal weight. Hover either to see it the other way round.
- **Usage alerts** — A toast when a window crosses a threshold or is about to reset, with configurable thresholds, reminders, accounts, and cadence
- **System tray** — Lives in the Windows tray area; left-click to toggle the popup, right-click to quit
- **Stats** — Input/output tokens, active sessions, message counts
- **7-day trend chart** — Daily usage sparkline per profile
- **Light / Dark / System theme** — Glassmorphism UI with backdrop blur
- **Multi-profile** — Add, remove, and switch between multiple provider profiles

## Limits and alerts

### Where the numbers come from

| Provider | Source |
|----------|--------|
| Claude (account) | Anthropic's OAuth usage endpoint, using the token Claude Code stores in `~/.claude/.credentials.json` |
| Codex (account) | ChatGPT's usage endpoint, using the token Codex stores in `~/.codex/auth.json`. Falls back to the last snapshot recorded in the rollout logs when the sign-in cannot be used. |
| z.ai (API) | The z.ai quota endpoint |

Token counts, sessions, and daily trends are read from local logs (`~/.claude/projects`, `~/.codex/sessions`, `~/.gemini`) and never leave the machine.

### When they are read

- **On opening the popup**, and when you press refresh. Refresh is an explicit ask and bypasses the cache.
- **On the alert engine's own schedule** in the background, so alerts still fire while the popup is closed. Default is every 15 minutes.

Readings are shared between the popup and the alert engine and cached for two minutes, so opening the popup repeatedly costs nothing. A provider that refuses is retried with a widening gap, up to 30 minutes.

> The usage endpoints above are private and undocumented. They rate limit frequent polling, and once limited they keep refusing until the polling stops, which shows up as **No limit data**. Keep the alert interval at a few minutes; the picker does not offer anything under a minute for this reason. These endpoints can also change without notice.

Because limits are sampled on a schedule rather than streamed, an alert can arrive up to one check interval late. Reset reminders are matched to a window rather than an exact minute: a reminder counts as due anywhere within one check interval of its mark, so "an hour before" lands near the hour. A reminder whose window went by unnoticed is dropped rather than delivered late.

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
  components/tray/          # Popup UI (TrayPopup, LimitWindows, StatCards, ...)
  components/alert/         # Toast overlay window (AlertOverlay, AlertToast)
  hooks/                    # Data fetching hooks
  lib/                      # Types, colors, formatting, limits, theme
  styles/                   # Global CSS with theme variables

src-tauri/src/              # Rust backend
  providers/                # Provider implementations
    claude.rs               # Claude Code (local ~/.claude)
    claude_api.rs           # Claude Admin API
    codex.rs                # Codex CLI (local ~/.codex)
    gemini.rs               # Gemini CLI (local ~/.gemini)
    zai.rs                  # z.ai (local %APPDATA%/zai)
    mod.rs                  # Provider trait, shared HTTP client
  alerts.rs                 # Limit polling, alert rules, toast window
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
- `~/.codex/` → Codex
- `~/.gemini/` → Gemini
- `%APPDATA%/zai/` → z.ai

A provider added after your config was written is offered once, and the offer is recorded, so a profile you removed is not added back on the next launch.

Additional profiles (including Claude API) can be added from the Settings panel. Settings also cover the theme, whether limits read as remaining or used, and the alert rules.

## License

MIT
