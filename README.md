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
