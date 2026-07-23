#!/bin/bash

set -euo pipefail
source "$(dirname "$0")/release-lib.sh"

require_command codesign
app_path="${1:-}"
[[ -d "$app_path/Contents" ]] || release_fail "pass a built Tooldeck.app path"
identity="${APPLE_SIGNING_IDENTITY:-}"
[[ -n "$identity" ]] ||
  release_fail "APPLE_SIGNING_IDENTITY must name a Developer ID Application identity"

for name in "${TOOLDECK_SIDECAR_NAMES[@]}"; do
  binary="$(sidecar_bundle_path "$app_path" "$name")"
  [[ -x "$binary" ]] || release_fail "bundled sidecar is missing: $binary"
  assert_self_contained_macho "$binary" "$name"
  codesign \
    --force \
    --options runtime \
    --timestamp \
    --sign "$identity" \
    "$binary"
  codesign --verify --strict --verbose=2 "$binary"
done

codesign \
  --force \
  --options runtime \
  --timestamp \
  --entitlements "$TOOLDECK_RELEASE_ROOT/src-tauri/Entitlements.plist" \
  --sign "$identity" \
  "$app_path"
codesign --verify --deep --strict --verbose=2 "$app_path"
release_note "signed and verified $app_path with hardened runtime"
