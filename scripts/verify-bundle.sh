#!/bin/bash

set -euo pipefail
source "$(dirname "$0")/release-lib.sh"

require_command codesign
app_path="${1:-}"
[[ -d "$app_path/Contents" ]] || release_fail "pass a built Tooldeck.app path"

for name in "${TOOLDECK_SIDECAR_NAMES[@]}"; do
  binary="$(sidecar_bundle_path "$app_path" "$name")"
  assert_self_contained_macho "$binary" "$name"
  codesign --verify --strict --verbose=2 "$binary"
  case "$name" in
    ffmpeg | ffprobe) "$binary" -version >/dev/null 2>&1 ;;
    *) "$binary" -v >/dev/null 2>&1 ;;
  esac || release_fail "$name did not execute from the installed bundle"
  release_note "verified bundled $name"
done

codesign --verify --deep --strict --verbose=2 "$app_path"
if [[ "${EXPECT_NOTARIZED:-0}" == "1" ]]; then
  spctl --assess --type exec --verbose=2 "$app_path"
fi
release_note "bundle verification passed"
