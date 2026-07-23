#!/bin/bash

set -euo pipefail
source "$(dirname "$0")/release-lib.sh"

require_command file
require_command lipo
require_command otool
require_command shasum

source_directory="${1:-${TOOLDECK_SIDECAR_SOURCE_DIR:-}}"
[[ -n "$source_directory" ]] ||
  release_fail \
    "pass a directory containing reviewed sidecars or set TOOLDECK_SIDECAR_SOURCE_DIR"
[[ -d "$source_directory" ]] ||
  release_fail "sidecar source directory does not exist: $source_directory"
[[ "${TOOLDECK_THIRD_PARTY_APPROVED:-0}" == "1" ]] ||
  release_fail \
    "set TOOLDECK_THIRD_PARTY_APPROVED=1 after reviewing THIRD_PARTY_NOTICES.md"

ffmpeg_source="$source_directory/ffmpeg"
[[ -x "$ffmpeg_source" ]] || release_fail "ffmpeg is missing or not executable"
ffmpeg_configuration="$("$ffmpeg_source" -version 2>&1 | head -n 4)"
if grep -q -- '--enable-gpl' <<<"$ffmpeg_configuration" &&
  [[ "${TOOLDECK_ALLOW_GPL_FFMPEG:-0}" != "1" ]]; then
  release_fail \
    "the supplied FFmpeg enables GPL components; provide an LGPL-compatible build or explicitly approve the GPL distribution impact"
fi

for name in "${TOOLDECK_SIDECAR_NAMES[@]}"; do
  source_path="$source_directory/$name"
  assert_self_contained_macho "$source_path" "$name"
  case "$name" in
    ffmpeg | ffprobe)
      "$source_path" -version >/dev/null 2>&1 ||
        release_fail "$name did not answer its version probe"
      ;;
    *)
      "$source_path" -v >/dev/null 2>&1 ||
        release_fail "$name did not answer its version probe"
      ;;
  esac
done

binary_directory="$TOOLDECK_RELEASE_ROOT/src-tauri/binaries"
mkdir -p "$binary_directory"
for name in "${TOOLDECK_SIDECAR_NAMES[@]}"; do
  destination="$(sidecar_source_path "$name")"
  cp "$source_directory/$name" "$destination"
  chmod 0755 "$destination"
  release_note \
    "staged $name ($(shasum -a 256 "$destination" | awk '{print $1}'))"
done

release_note "all reviewed sidecars are staged for $TOOLDECK_TARGET_TRIPLE"
