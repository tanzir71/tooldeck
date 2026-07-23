#!/bin/bash

set -euo pipefail

TOOLDECK_RELEASE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLDECK_TARGET_TRIPLE="${TOOLDECK_TARGET_TRIPLE:-aarch64-apple-darwin}"
TOOLDECK_SIDECAR_NAMES=(
  ffmpeg
  ffprobe
  pdfinfo
  pdfseparate
  pdftocairo
  pdftoppm
  pdftotext
  pdfunite
)

release_note() {
  printf 'Tooldeck release: %s\n' "$*"
}

release_fail() {
  printf 'Tooldeck release error: %s\n' "$*" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 ||
    release_fail "required command '$1' is unavailable"
}

sidecar_source_path() {
  printf '%s/src-tauri/binaries/%s-%s' \
    "$TOOLDECK_RELEASE_ROOT" "$1" "$TOOLDECK_TARGET_TRIPLE"
}

sidecar_bundle_path() {
  printf '%s/Contents/MacOS/%s' "$1" "$2"
}

assert_self_contained_macho() {
  local binary="$1"
  local label="$2"
  [[ -f "$binary" ]] || release_fail "$label is missing at $binary"
  [[ -x "$binary" ]] || release_fail "$label is not executable"
  file "$binary" | grep -Eq 'Mach-O.*(arm64|universal binary)' ||
    release_fail "$label is not an arm64 Mach-O executable"
  lipo -archs "$binary" | tr ' ' '\n' | grep -qx arm64 ||
    release_fail "$label does not contain an arm64 slice"

  local dependency
  while IFS= read -r dependency; do
    dependency="${dependency#"${dependency%%[![:space:]]*}"}"
    dependency="${dependency%% *}"
    [[ -z "$dependency" ]] && continue
    case "$dependency" in
      /System/Library/* | /usr/lib/*) ;;
      *)
        release_fail \
          "$label depends on non-system library $dependency; provide a self-contained build"
        ;;
    esac
  done < <(otool -L "$binary" | tail -n +2)
}

notary_credentials_available() {
  [[ -n "${NOTARYTOOL_PROFILE:-}" ]] ||
    [[ -n "${APPLE_API_KEY_PATH:-}" &&
      -n "${APPLE_API_KEY:-}" &&
      -n "${APPLE_API_ISSUER:-}" ]] ||
    [[ -n "${APPLE_ID:-}" &&
      -n "${APPLE_PASSWORD:-}" &&
      -n "${APPLE_TEAM_ID:-}" ]]
}
