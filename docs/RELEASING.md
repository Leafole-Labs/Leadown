# Releasing abstract

## Cutting a release

1. Bump `version` in `Cargo.toml`, update `Cargo.lock`
   (`cargo update -p abstract-editor`), and fill in the `CHANGELOG.md` entry.
2. Commit, tag `vX.Y.Z`, push the tag. The `release` workflow builds all
   targets, packages them, and publishes a GitHub Release with checksums.

Artifacts per release: Linux x86_64 and aarch64 (`tar.gz`, `.deb`, `.rpm`,
`.AppImage`), macOS arm64 + Intel (`dmg`, `.app.zip`, `tar.gz`), Windows
(`exe`, `zip`), plus `SHA256SUMS`.

## macOS signing and notarization

Without secrets the workflow produces ad-hoc signed builds. To ship signed,
notarized builds, set these repository secrets:

- `MACOS_CERTIFICATE_P12` — Developer ID Application certificate exported
  from Keychain Access as `.p12`, then `base64 -i cert.p12 | pbcopy`.
- `MACOS_CERTIFICATE_PASSWORD` — the export password of that `.p12`.
- `MACOS_SIGNING_IDENTITY` — e.g. `Developer ID Application: Name (TEAMID)`.
- `APPLE_ID` — Apple ID used for notarization.
- `APPLE_TEAM_ID` — 10-character team identifier.
- `APPLE_APP_PASSWORD` — app-specific password from appleid.apple.com
  (Sign-In and Security → App-Specific Passwords).

The workflow imports the cert into a temporary keychain, stores a notarytool
profile named `abstract-notary`, signs with `--options runtime --timestamp`,
submits the app and dmg to the notary service, and staples both tickets.

## AUR bump after a release

Each AUR package is its own repo (`packaging/aur/README.md` has the full flow):

1. `pkgver=<new>` and `pkgrel=1` in `packaging/aur/abstract-editor/PKGBUILD`
   and `packaging/aur/abstract-editor-bin/PKGBUILD`.
2. `updpkgsums` inside a checkout of the AUR repo (needs the tag and release
   assets published first).
3. `makepkg --printsrcinfo > .SRCINFO`, `makepkg -si` to verify.
4. Commit `PKGBUILD` + `.SRCINFO` and push to
   `ssh://aur@aur.archlinux.org/<pkgname>.git`.
