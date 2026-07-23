# Tooldeck third-party release notice

Tooldeck is local-first software built with Tauri, Rust, React, SQLite, and
Apple system frameworks. JavaScript and Rust dependency license metadata lives
in `package-lock.json` and `src-tauri/Cargo.lock`.

## Release sidecars are supplied separately

This repository does **not** commit or silently copy the developer machine's
Homebrew tools. A release operator must provide reviewed, self-contained
Apple Silicon binaries to `scripts/prepare-sidecars.sh`.

The Tooldeck 1.0 runtime expects:

- FFmpeg and FFprobe for focused video operations.
- Poppler command-line utilities (`pdfinfo`, `pdfseparate`, `pdftocairo`,
  `pdftoppm`, `pdftotext`, and `pdfunite`) for the current PDF engine.

The preparation script verifies architecture, rejects non-system dynamic
library dependencies, records a SHA-256 in its output, and refuses to continue
until `TOOLDECK_THIRD_PARTY_APPROVED=1` is set.

## FFmpeg licensing gate

Prefer a self-contained FFmpeg build whose selected configuration and codecs
are compatible with the LGPL. The release preflight rejects a build containing
`--enable-gpl` unless the release owner explicitly sets
`TOOLDECK_ALLOW_GPL_FFMPEG=1`.

That override is not a legal conclusion. Enabling GPL components changes the
distribution obligations for the binary and may change the obligations for
the product as a whole. The release owner must preserve the exact FFmpeg
configuration, source offer/source location, copyright notices, and license
text that apply to the selected build.

## Poppler licensing gate

The current PDF command-line sidecars are from Poppler, which is commonly
distributed under GPL terms. Before shipping, the release owner must have the
planned aggregation, notices, corresponding-source process, and product
licensing reviewed. `TOOLDECK_THIRD_PARTY_APPROVED=1` acknowledges that human
review; it does not replace it.

## OCR choice

Tooldeck uses the Apple Vision framework through direct Rust FFI for offline
OCR. It therefore does not bundle Tesseract or language packs in the 1.0
release candidate. This keeps OCR on-device and avoids a second executable
sidecar while retaining the handoff's documented no-Swift Vision path.

This document is an engineering release gate, not legal advice.
