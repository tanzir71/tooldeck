# Tooldeck — M0 scaffold

Starter files for the Codex build loop. Drop these into the repo layout from the handoff (§3.1) and use them as the spine for Milestone 0.

## What's here
- **`ACCEPTANCE.md`** — the living checklist the loop updates every milestone (mirrors handoff §11–§12). Move this to the repo root. `make verify` must pass before checking items off.
- **`Makefile`** — `make verify` is the single loop gate (fmt-check, clippy, tsc, eslint, cargo test, e2e smoke). Move to repo root. Run `make help` for targets.
- **`src-tauri/src/registry.rs`** — the Rust tool-registry stub: `Tool` trait, `ToolMeta`, declarative `FormSchema`, `Registry`, a working example tool (`image-resize`), Tauri command signatures, and tests. This is the pattern every tool follows (§15.1).
- **`src/lib/registry.ts`** — the matching frontend types + IPC wrappers + palette fuzzy-search + persona profiles (§16.1, §16.4).

## First moves for the agent (M0)
1. `npm create tauri-app@latest` (React + TS + Vite template), then reconcile the folder layout with handoff §3.1.
2. Drop in `registry.rs` and `registry.ts`; register the `image-resize` stub end-to-end so the palette can find and run it.
3. Build the three surfaces (§16.1): command palette, main window (rail/Home/workspace), menu-bar dropdown — all generated from `fetchCatalog()`.
4. Wire design tokens (§16.6), light/dark, SQLite + first migration.
5. Make `make verify` green. Check off M0 in `ACCEPTANCE.md`. Commit.

## Notes
- Keep `registry.rs` and `registry.ts` in sync, or auto-generate the TS from Rust with `tauri-specta`/`ts-rs` once the shape stabilizes.
- Don't hardcode tool lists anywhere in the UI — everything renders from the catalog.
- Sidecars (ffmpeg/ffprobe/tesseract) aren't needed until M5/M6; see `make sidecars` and handoff §9.4 for the signing/notarization caveat.
