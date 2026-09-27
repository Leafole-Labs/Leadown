# Releasing Leadown

1. Bump `version` in `Cargo.toml` and `tauri.conf.json`.
2. Update `CHANGELOG.md` with the new version's changes.
3. Commit: `git commit -am "release: v<version>"`.
4. Tag: `git tag v<version>`.
5. Push: `git push && git push --tags`.
6. The `release` workflow builds for all platforms and publishes a GitHub
   Release with all artifacts attached.

## macOS signing & notarization

The workflow imports the signing certificate into a temporary keychain,
stores notary credentials in the `leadown-notary` keychain profile, and
signs with `--options runtime --timestamp`, then notarizes and staples both
the `.app` and the `.dmg`.

## Linux packaging

`cargo deb` and `cargo-generate-rpm` produce the `.deb` and `.rpm` from the
`Cargo.toml` metadata. The `.AppImage` is built by `scripts/package.sh`
via `appimagetool`.

## AUR packages

1. `pkgver=<new>` and `pkgrel=1` in `packaging/aur/leadown/PKGBUILD`
   and `packaging/aur/leadown-bin/PKGBUILD`.
2. `updpkgsums` (needs the tag and release assets published first).
3. `makepkg --printsrcinfo > .SRCINFO`, `makepkg -si` to verify, commit, push.
