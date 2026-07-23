# Tooldeck

Tooldeck is a local-first macOS productivity workspace for notes, time tracking,
document utilities, and focused media tools. The application uses Tauri v2,
Rust, SQLite, React, TypeScript, and Tailwind CSS.

## Development

Requirements: macOS 14+, Apple Silicon, Node.js, npm, and the Rust toolchain.

```sh
npm install
make verify
npm run tauri dev
```

The Rust registry is the source of truth for tools. The command palette, Home,
category rail, and workspace all consume the same `tool_catalog` IPC command.

See [TOOLDECK_CODEX_HANDOFF.md](./TOOLDECK_CODEX_HANDOFF.md) for product scope
and [ACCEPTANCE.md](./ACCEPTANCE.md) for milestone status.

## Release

Public releases use a separate Tauri configuration that embeds eight reviewed
Apple Silicon sidecars, signs each executable with hardened runtime, signs the
app and DMG, notarizes with Apple, staples the ticket, and verifies the
installed bundle.

Run `make release-audit` for an exact readiness report. See
[RELEASE.md](./RELEASE.md) for the credential and clean-machine flow and
[THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md) for the FFmpeg, Poppler, and
OCR release decisions. Development Homebrew binaries are never copied into a
release automatically.
