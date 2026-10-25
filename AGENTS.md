# AGENTS.md — ClipKit (rust-clipkit)

ClipKit is a native desktop GUI (Rust + [Iced](https://github.com/iced-rs/iced) 0.12, Tokio) for a command-line media download engine (yt-dlp, managed in `crates/yt_dlp_backend`) plus an ffmpeg-based transcoder with HDR/Dolby Vision-aware presets. It targets Windows, macOS and Linux.

## Current state (2026-10-20)

- Builds and runs (`cargo build` succeeds on Linux). There is **1 unit test** in the whole workspace and **no CI on push/PR** — only a tag-triggered release workflow.
- The last release run (June 2026) failed: both Linux bundle jobs failed in *Build and Bundle (Linux)*, and the macOS Intel job uses the retired `macos-13` runner.
- `Cargo.lock` is not committed, so builds are not reproducible.
- The engine updater downloads and runs a binary from GitHub releases **without verifying its checksum**.
- `cargo clippy` reports 10 warnings (dead code, `map_or` simplifications, a complex type).

## Commands

Linux build dependencies (from the release workflow): `pkg-config libxkbcommon-dev libwayland-dev wayland-protocols libx11-dev libxcb1-dev libxcb-shape0-dev libxcb-xfixes0-dev libxcb-render0-dev libvulkan-dev libgles2-mesa-dev`.

```bash
cargo build                      # debug build of the whole workspace
cargo run --release -p gui       # the app; binary is target/release/clipkit
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings   # goal; currently fails with 10 warnings
cargo fmt --all -- --check
scripts/build.sh | scripts\build.bat                    # build-and-run helpers
scripts/bundle_{linux,macos}.sh, scripts/bundle_windows.bat  # release bundles (AppImage/tar.gz, .app/.dmg, zip)
```

Windows with Application Control policies: build with `CARGO_TARGET_DIR=C:\ProgramData\clipkit-build` (what `build.bat` does).

## Layout

| Crate | Responsibility |
|---|---|
| `crates/core` | Config persistence, job models (`DownloadJob`, `TranscodeJob`), presets, events |
| `crates/yt_dlp_backend` | GitHub release listing, engine binary download/activation, option mapping, download process + progress parsing, self-update |
| `crates/transcoder` | ffmpeg detection/management, transcode engine, HDR/DV handling |
| `crates/gui` | Iced app state (`app.rs`), panes (`download`, `manager`, `transcoder`, `settings`), theme, bundled Inter font |

Events flow from Tokio tasks → `tokio::sync::broadcast` → Iced subscription → UI messages. The UI thread never blocks. Config lives in the platform config dir (`%APPDATA%\clipkit`, `~/.config/clipkit`); engine binaries in `%LOCALAPPDATA%\clipkit\bin\<version>` or `~/.local/share/clipkit/bin/<version>`.

## Rules

- Keep process I/O and HTTP in `core`/backend crates, never in `gui` view code; the GUI talks to them through events and messages.
- Build command lines as argument vectors (`Command::arg`), never by concatenating a shell string. URLs and paths come from users.
- Any downloaded executable must be checksum-verified before it is marked executable or run (yt-dlp publishes `SHA2-256SUMS` with each release).
- Parsing of engine progress output and the preset → ffmpeg argument mapping are pure functions; give them unit tests with captured sample output.
- Don't commit `target/`, downloaded engine binaries or bundles from `dist/`.

## Suggested first tasks

1. Commit `Cargo.lock`; add a CI workflow on push/PR running `fmt --check`, `clippy -D warnings`, `test` and a Linux build.
2. Fix the clippy warnings, then add tests for `option_map`, progress parsing and `presets`.
3. Verify engine downloads against the release checksums.
4. Fix the Linux release job and replace `macos-13` with a supported Intel macOS runner (or drop that target).

## Git workflow

Never push to `main`; branch, run the commands above, and open a PR listing what you ran. Releases are cut by pushing a `vX.Y.Z` tag.

## Future SLOPOS/PARAMA integration and Pi Zero 2 W efficiency goals

**Status: PLANNED / NOT IMPLEMENTED / NOT A CURRENT RELEASE GATE.** This repository is an independently authored candidate for the **optional first-party SLOPOS Downloads/media application**. Preserve its current Rust, Iced 0.12, Tokio, yt-dlp and FFmpeg architecture, license, existing host support, current milestones and evidence requirements; do not rewrite the UI or core engine merely to target SLOPOS. Its GitHub non-fork status is not a substitute for an actual source/asset/license audit. SLOPOS first-party branding means curated, independently verified original software, not automatic default preinstallation.

- **Linux/SLOPOS first:** Real SLOPOS Linux desktop launcher, open/save picker and permissioned target path, status/notifications, XDG menus/clipboard and responsive operations under slow network and microSD. Reuse host ffmpeg instead of a duplicate transcoder library; disclose yt-dlp/ffmpeg runtime version, provenance and licenses.
- **Raspberry Pi Zero 2 W efficiency (4× Cortex-A53, 512 MB shared RAM):** Streaming downloads and transcoding with bounded producer/consumer queues, capped chunk/buffer/thumbnail memory, backpressure, incremental progress, cancellation and restart/resume where supported. Do not buffer whole media files, start ffmpeg or contact update servers at desktop idle. Hardware encoding is opportunistic, never required. On Pi Zero 2 W prove the Iced renderer, codecs and yt-dlp runtime actually work before claiming a native package.
- **Native PARAMA/SURYA future target:** Far-future native Rust/Iced port requiring PARAMA process/thread, async event/poller, TLS/network/file/clock, SURYA input/window/graphics, and carefully ported external yt-dlp/Python and FFmpeg or native compatible alternatives. Never claim present Wine/Linux binary or yt-dlp support on PARAMA.
- **Real device/VM acceptance (NOT_RUN):** At 720p on actual Pi Zero 2 W: bounded resident memory and idle CPU with no download, progress/cancel on a slow real stream, failed/full microSD, interrupted/resumed file and correct byte-level output. Record 4-core CPU and process-tree PSS; codecs may be unavailable or slow.
- **Proof and governance:** Record target OS/ABI, library/runtime/model/codec versions, application SHA, actual Pi memory `MemAvailable`, process tree PSS/peak RSS, idle CPU, cold/warm first-usable latency, visible UI response p95, cancellation and correctness, missing features and failure cases. Aarch64 cross-compilation or QEMU is not physical hardware proof; PARAMA requires a separate native port proof, never implied by SLOPOS/Linux acceptance. Keep optional heavy runtimes and model downloads off the 512 MB core SLOPOS image; permit offloading only after informed user consent. Do not weaken correctness, privacy or project release gates to hit a memory target.
