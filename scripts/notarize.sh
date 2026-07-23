#!/bin/bash

set -euo pipefail
source "$(dirname "$0")/release-lib.sh"

require_command spctl
require_command xcrun

dmg_path="${1:-}"
[[ -f "$dmg_path" ]] || release_fail "pass the signed Tooldeck DMG path"
notary_arguments=()
if [[ -n "${NOTARYTOOL_PROFILE:-}" ]]; then
  notary_arguments=(--keychain-profile "$NOTARYTOOL_PROFILE")
elif [[ -n "${APPLE_API_KEY_PATH:-}" &&
  -n "${APPLE_API_KEY:-}" &&
  -n "${APPLE_API_ISSUER:-}" ]]; then
  notary_arguments=(
    --key "$APPLE_API_KEY_PATH"
    --key-id "$APPLE_API_KEY"
    --issuer "$APPLE_API_ISSUER"
  )
elif [[ -n "${APPLE_ID:-}" &&
  -n "${APPLE_PASSWORD:-}" &&
  -n "${APPLE_TEAM_ID:-}" ]]; then
  notary_arguments=(
    --apple-id "$APPLE_ID"
    --password "$APPLE_PASSWORD"
    --team-id "$APPLE_TEAM_ID"
  )
else
  release_fail \
    "configure NOTARYTOOL_PROFILE, App Store Connect API, or Apple ID credentials"
fi

xcrun notarytool submit "$dmg_path" "${notary_arguments[@]}" --wait
xcrun stapler staple "$dmg_path"
xcrun stapler validate "$dmg_path"
spctl \
  --assess \
  --type open \
  --context context:primary-signature \
  --verbose=2 \
  "$dmg_path"
release_note "notarization ticket is accepted and stapled"
