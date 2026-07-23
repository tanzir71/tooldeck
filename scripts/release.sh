#!/bin/bash

set -euo pipefail
source "$(dirname "$0")/release-lib.sh"

dry_run=0
if [[ "${1:-}" == "--dry-run" ]]; then
  dry_run=1
fi

if ((dry_run == 0)); then
  "$TOOLDECK_RELEASE_ROOT/scripts/release-audit.sh"
else
  for name in "${TOOLDECK_SIDECAR_NAMES[@]}"; do
    assert_self_contained_macho "$(sidecar_source_path "$name")" "$name"
  done
fi

release_note "building the Apple Silicon app bundle"
env \
  -u APPLE_ID \
  -u APPLE_PASSWORD \
  -u APPLE_TEAM_ID \
  -u APPLE_API_KEY \
  -u APPLE_API_ISSUER \
  -u APPLE_API_KEY_PATH \
  APPLE_SIGNING_IDENTITY=- \
  npm run tauri build -- \
  --config src-tauri/tauri.release.conf.json \
  --bundles app

app_path="$TOOLDECK_RELEASE_ROOT/src-tauri/target/release/bundle/macos/Tooldeck.app"
if ((dry_run == 0)); then
  "$TOOLDECK_RELEASE_ROOT/scripts/sign.sh" "$app_path"
fi
"$TOOLDECK_RELEASE_ROOT/scripts/verify-bundle.sh" "$app_path"

dmg_path="$TOOLDECK_RELEASE_ROOT/release/Tooldeck-1.0.0-aarch64.dmg"
if ((dry_run == 0)); then
  "$TOOLDECK_RELEASE_ROOT/scripts/package-dmg.sh" "$app_path" "$dmg_path"
  "$TOOLDECK_RELEASE_ROOT/scripts/notarize.sh" "$dmg_path"
else
  APPLE_SIGNING_IDENTITY= \
    "$TOOLDECK_RELEASE_ROOT/scripts/package-dmg.sh" "$app_path" "$dmg_path"
fi
release_note "release pipeline completed"
