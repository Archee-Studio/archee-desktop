# Archee Studio for the desktop

A native window around [Archee Studio](https://archee.studio), built with [Tauri](https://tauri.app). It hosts the web app and adds what a desktop window needs: a draggable title bar, zoom keys and opening outside links in your browser.

## Install

- **Download:** [archee.studio/download](https://archee.studio/download/) for macOS (Apple silicon and Intel) and Windows.
- **Homebrew:** `brew install --cask archee-studio/tap/archee-desktop`

Each [release](https://github.com/Archee-Studio/archee-desktop/releases) lists a SHA-256 checksum for every installer.

## Build from source

You need [Rust](https://rustup.rs) and [Bun](https://bun.sh).

```bash
bun install
bun run build    # the installer for your system, in src-tauri/target/release/bundle
```

This repository is published from the Archee Studio source with each release; pull requests are not merged here. Report a security issue as described in [SECURITY.md](SECURITY.md).

## License

[Apache License 2.0](LICENSE).
