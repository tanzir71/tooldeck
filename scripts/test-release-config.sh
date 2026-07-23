#!/bin/bash

set -euo pipefail
source "$(dirname "$0")/release-lib.sh"

for script in "$TOOLDECK_RELEASE_ROOT"/scripts/*.sh; do
  bash -n "$script"
done
plutil -lint "$TOOLDECK_RELEASE_ROOT/src-tauri/Entitlements.plist" >/dev/null
node -e '
  const fs = require("fs");
  const root = process.argv[1];
  const base = JSON.parse(fs.readFileSync(`${root}/src-tauri/tauri.conf.json`));
  const release = JSON.parse(
    fs.readFileSync(`${root}/src-tauri/tauri.release.conf.json`),
  );
  if (base.version !== "1.0.0") throw new Error("release version is not 1.0.0");
  if (!base.bundle.macOS.hardenedRuntime) {
    throw new Error("hardened runtime is disabled");
  }
  if (base.bundle.macOS.entitlements !== "./Entitlements.plist") {
    throw new Error("entitlements are not wired");
  }
  if (release.bundle.externalBin.length !== 8) {
    throw new Error("the release must contain eight sidecars");
  }
' "$TOOLDECK_RELEASE_ROOT"

set +e
audit_output="$(
  env \
    -u APPLE_SIGNING_IDENTITY \
    -u NOTARYTOOL_PROFILE \
    -u APPLE_API_KEY_PATH \
    -u APPLE_API_KEY \
    -u APPLE_API_ISSUER \
    -u APPLE_ID \
    -u APPLE_PASSWORD \
    -u APPLE_TEAM_ID \
    -u TOOLDECK_THIRD_PARTY_APPROVED \
    "$TOOLDECK_RELEASE_ROOT/scripts/release-audit.sh" 2>&1
)"
audit_status=$?
set -e
[[ "$audit_status" == "2" ]] ||
  release_fail "credential-free audit should exit 2, got $audit_status"
grep -q 'BLOCKED' <<<"$audit_output" ||
  release_fail "credential-free audit did not explain its blockers"
release_note "release configuration and blocker reporting passed"
