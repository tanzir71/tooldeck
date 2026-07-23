# Tooldeck 1.0 release runbook

Tooldeck ships as an Apple Silicon, macOS 14+ direct-download DMG. Development
builds use ad-hoc signing. A public release requires a Developer ID Application
certificate, Apple notarization credentials, and reviewed self-contained
sidecars.

## 1. Supply reviewed sidecars

Place unsuffixed executable files in a private source directory:

```text
ffmpeg
ffprobe
pdfinfo
pdfseparate
pdftocairo
pdftoppm
pdftotext
pdfunite
```

After completing the review in `THIRD_PARTY_NOTICES.md`:

```sh
TOOLDECK_THIRD_PARTY_APPROVED=1 \
  make sidecars SIDECAR_SOURCE=/absolute/path/to/reviewed-sidecars
```

The script rejects the GPL-enabled Homebrew FFmpeg by default, rejects binaries
without an arm64 slice, and rejects references to Homebrew or other non-system
dynamic libraries.

## 2. Configure Apple credentials

Set the installed Developer ID identity:

```sh
export APPLE_SIGNING_IDENTITY="Developer ID Application: Example, Inc. (TEAMID)"
```

Choose one notarization method:

- `NOTARYTOOL_PROFILE` for a Keychain profile created with
  `xcrun notarytool store-credentials`.
- `APPLE_API_KEY_PATH`, `APPLE_API_KEY`, and `APPLE_API_ISSUER` for App Store
  Connect API authentication.
- `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID` for an app-specific
  password.

Do not commit certificates, passwords, API keys, or private sidecar builds.

## 3. Audit, build, sign, notarize, and staple

```sh
TOOLDECK_THIRD_PARTY_APPROVED=1 make release-audit
TOOLDECK_THIRD_PARTY_APPROVED=1 make build
```

The release pipeline:

1. validates every staged sidecar;
2. builds `Tooldeck.app` with Tauri's `externalBin` release overlay;
3. signs every sidecar with hardened runtime and verifies it;
4. signs the app with the minimal direct-distribution entitlements;
5. executes every sidecar from `Contents/MacOS`;
6. creates and signs `release/Tooldeck-1.0.0-aarch64.dmg`;
7. submits with `notarytool --wait`, staples, and validates the ticket; and
8. runs `codesign --verify`, `spctl`, and the installed-bundle sidecar smoke.

For a credential-free packaging rehearsal after sidecars are staged:

```sh
make package-dry-run
```

The rehearsal is ad-hoc signed and intentionally cannot satisfy Gatekeeper.

## 4. Clean-machine acceptance

On a Mac without Homebrew:

1. download the notarized DMG through a browser;
2. open it without bypassing Gatekeeper;
3. drag Tooldeck to Applications and launch it;
4. complete or skip onboarding;
5. run PDF merge/text extraction and video-to-GIF;
6. verify Output Center completes both jobs; and
7. verify `spctl -a -t exec -vv /Applications/Tooldeck.app` reports accepted.

Record the DMG SHA-256, notarization submission ID, certificate identity,
sidecar hashes, FFmpeg configuration, and clean-machine result in the release
record.

References:

- Tauri macOS signing:
  https://v2.tauri.app/distribute/sign/macos/
- Tauri external binaries:
  https://v2.tauri.app/develop/sidecar/
- Apple notarization workflow:
  https://developer.apple.com/documentation/security/customizing-the-notarization-workflow
