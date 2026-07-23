# Release sidecars

This directory is populated by `scripts/prepare-sidecars.sh`. Release binaries
are intentionally not committed.

Tooldeck 1.0 requires self-contained Apple Silicon builds of:

- `ffmpeg` and `ffprobe`
- `pdfinfo`, `pdfseparate`, `pdftocairo`, `pdftoppm`, `pdftotext`, and
  `pdfunite`

Each staged file uses Tauri's required `-aarch64-apple-darwin` suffix. The
preflight rejects non-Mach-O files, non-arm64 files, non-system dylib
dependencies, and GPL-enabled FFmpeg builds unless a human explicitly
overrides the license gate.
