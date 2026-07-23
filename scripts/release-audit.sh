#!/bin/bash

set -euo pipefail
source "$(dirname "$0")/release-lib.sh"

require_command codesign
require_command file
require_command hdiutil
require_command lipo
require_command otool
require_command spctl
require_command xcrun

failures=0
for name in "${TOOLDECK_SIDECAR_NAMES[@]}"; do
  path="$(sidecar_source_path "$name")"
  if [[ ! -x "$path" ]]; then
    release_note "BLOCKED · reviewed $name is not staged"
    failures=$((failures + 1))
    continue
  fi
  if ! (
    assert_self_contained_macho "$path" "$name"
  ); then
    failures=$((failures + 1))
  else
    release_note "READY · $name is self-contained arm64 Mach-O"
  fi
done

if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  if security find-identity -v -p codesigning |
    grep -Fq "$APPLE_SIGNING_IDENTITY"; then
    release_note "READY · Developer ID identity is installed"
  else
    release_note "BLOCKED · APPLE_SIGNING_IDENTITY is not installed"
    failures=$((failures + 1))
  fi
else
  release_note "BLOCKED · APPLE_SIGNING_IDENTITY is not set"
  failures=$((failures + 1))
fi

if notary_credentials_available; then
  release_note "READY · notarization credentials are configured"
else
  release_note \
    "BLOCKED · configure NOTARYTOOL_PROFILE, App Store Connect API, or Apple ID notarization credentials"
  failures=$((failures + 1))
fi

if [[ "${TOOLDECK_THIRD_PARTY_APPROVED:-0}" == "1" ]]; then
  release_note "READY · third-party release review is acknowledged"
else
  release_note \
    "BLOCKED · review THIRD_PARTY_NOTICES.md and set TOOLDECK_THIRD_PARTY_APPROVED=1"
  failures=$((failures + 1))
fi

if ((failures > 0)); then
  release_note "$failures release prerequisite(s) remain"
  exit 2
fi
release_note "all release prerequisites are ready"
