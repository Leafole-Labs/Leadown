# AUR packaging

Two packages live here:

- `abstract-editor/` — builds `abstract` from the GitHub source tarball.
- `abstract-editor-bin/` — repackages the prebuilt Linux tarball from GitHub Releases.

Each AUR package is a separate git repository, so the contents of each directory
(`PKGBUILD` + `.SRCINFO`) get copied into its own repo.

## Publishing for the first time

```bash
git clone ssh://aur@aur.archlinux.org/abstract-editor.git
cd abstract-editor
cp /path/to/abstract/packaging/aur/abstract-editor/PKGBUILD .
updpkgsums                      # replace the SKIP checksums
makepkg --printsrcinfo > .SRCINFO
makepkg -si                     # build + install locally to test
namcap PKGBUILD *.pkg.tar.zst   # optional lint
git add PKGBUILD .SRCINFO
git commit -m "abstract-editor 0.1.0"
git push
```

Same flow for `abstract-editor-bin` with
`ssh://aur@aur.archlinux.org/abstract-editor-bin.git`.

## Bumping on a new release

1. `pkgver=<new>` in both PKGBUILDs, reset `pkgrel=1`.
2. `updpkgsums` (needs the tag and release assets published first).
3. `makepkg --printsrcinfo > .SRCINFO`, `makepkg -si` to verify, commit, push.
