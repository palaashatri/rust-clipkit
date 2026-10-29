# ClipKit — Interface & Visual Tour

ClipKit is redesigned around Apple Human Interface Guidelines with a brand identity centered on translucent 3D sculpted coral icons and dark ceramic / translucent slate styling.

---

## 1. Download Workspace

The primary workspace for downloading media across hundreds of video and audio sources.

![Download Pane](screenshots/01-download-pane.png)

- **Quick Action Bar:** Single-click download URL input with `.on_submit` keyboard execution (press Return/Enter to download).
- **Format Selection:** Easily switch between `Video + Audio`, `Audio Only`, or custom container profiles (`MP4`, `MKV`, `WebM`).
- **Sidebar Identity:** Sculpted 3D coral icon positioned under macOS window traffic lights with native 52px inset spacing.
- **Engine Pill:** Live indicator showing active managed or system engine version (`yt-dlp`).

---

## 2. Advanced Download Configuration

Expandable fine-grained engine control for technical workflows.

![Advanced Download Options](screenshots/02-download-advanced.png)

- **Quality & Resolution:** Choose from `Best`, `4K`, `1440p`, `1080p`, down to `144p`.
- **Metadata & Chapters:** One-click toggles for embedding thumbnails, complete video metadata, and SponsorBlock chapter skipping.
- **Subtitles & Closed Captions:** Automated subtitle download, language filtering (e.g. `en, es, fr`), embedding directly into output containers, and auto-generated captions support.

---

## 3. HDR-Aware Transcoder

Convert and remux media files with built-in presets and tone-mapping intelligence.

![Transcoder Pane](screenshots/03-transcoder-pane.png)

- **Input / Output Selection:** Native system file picker for input and output paths.
- **Preset Catalog:** Built-in profiles for universal playback, ProRes archive, audio extraction, and web publishing.
- **HDR & Dolby Vision Handling:** Intelligent tone-mapping to SDR or preservation of wide color gamut metadata (`bt2020` / `smpte2084`).
- **Command Preview:** Inspect exact FFmpeg argument vectors before executing jobs.

---

## 4. Engine & Dependency Management

Manage engine binaries safely with cryptographic checksum verification.

![Engine Management](screenshots/04-settings-engine.png)

- **Automated Discovery:** Fetch official engine releases directly from GitHub.
- **SHA-256 Checksum Verification:** Every binary is validated against release `SHA2-256SUMS` prior to being marked executable.
- **FFmpeg Integration:** Auto-detects local system installations or downloads platform archives with verified SHA-256 hashes.

---

## 5. Settings & About Showcase

Application preferences, appearance toggles, and project metadata.

![About Showcase](screenshots/05-settings-about.png)

- **Sculpted 3D Icon:** 64x64 showcase of the brand identity icon.
- **Instant Theme Switching:** Live theme selection (Dark, Light) with immediate visual feedback.
- **Open Source Licensing:** Transparent MIT licensing information.

---

## 6. Ceramic Light Theme

High-contrast, light ceramic theme designed for bright workspaces.

![Ceramic Light Mode](screenshots/06-light-mode.png)

- **WCAG AA Compliance:** Slate-600 secondary typography with 5.9:1 contrast ratio against white cards.
- **Subtle Borders & Peach Tints:** Delicate 0.5px card borders with peach selection pills and brand coral accents.
