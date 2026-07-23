#!/bin/bash

set -euo pipefail
source "$(dirname "$0")/release-lib.sh"

require_command codesign
require_command hdiutil

app_path="${1:-}"
output_path="${2:-$TOOLDECK_RELEASE_ROOT/release/Tooldeck-1.0.0-aarch64.dmg}"
[[ -d "$app_path/Contents" ]] || release_fail "pass a built Tooldeck.app path"

staging="$(mktemp -d "${TMPDIR:-/tmp}/tooldeck-dmg.XXXXXX")"
cleanup() {
  rm -rf "$staging"
}
trap cleanup EXIT

cp -R "$app_path" "$staging/Tooldeck.app"
ln -s /Applications "$staging/Applications"
mkdir -p "$(dirname "$output_path")"
hdiutil create \
  -volname "Tooldeck" \
  -srcfolder "$staging" \
  -ov \
  -format UDZO \
  "$output_path"

if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  codesign \
    --force \
    --timestamp \
    --sign "$APPLE_SIGNING_IDENTITY" \
    "$output_path"
  codesign --verify --strict --verbose=2 "$output_path"
fi
release_note "created $output_path"
