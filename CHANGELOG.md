# Changelog

All notable changes to abstract are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.1] - 2026-09-26

### Added
- Linux aarch64 builds (`tar.gz`, `.deb`, `.AppImage`, `.rpm`).
- `.AppImage` and `.rpm` packages for Linux x86_64.
- macOS builds are signed with a Developer ID and notarized when the signing
  secrets are configured in the release workflow; unsigned (ad-hoc) builds are
  still produced otherwise.
- `.SRCINFO` committed next to each AUR `PKGBUILD`.
- This changelog.

### Changed
- AUR `abstract-editor` / `abstract-editor-bin` bumped to `pkgrel=2` for the
  re-tagged v0.1.0.

## [0.1.0] - 2026-09-26

First public release.

### Added
- Local-first markdown notes editor rendered on the GPU with GPUI, with a
  sidebar of spaces and notes, a first-run tour and light/dark themes.
- Markdown analysis: headings, emphasis, fenced code blocks with syntax
  highlighting, painted bullets and clickable task checkboxes.
- `[[wiki-links]]` with autocomplete, Ctrl/Cmd+click to open or create the
  target note, and a backlinks panel under the editor.
- Global note search palette (Ctrl/Cmd+P).
- Live reload when the space folder changes on disk.
- i18n (English, pt-BR) with locale detection and an in-app switch.
- Bundled Noto Sans / Noto Sans Mono fonts.
- macOS: native traffic lights, app icon, macOS keybindings and graceful quit.
- Windows: native window control areas, icon embedded in the `.exe`.
- Linux: Wayland and X11 support; `.desktop` entry and icon.
- Release artifacts: macOS `.dmg` / `.app.zip` (Apple Silicon and Intel),
  Linux x86_64 `tar.gz` and `.deb`, Windows x86_64 `.exe` and `.zip`,
  `SHA256SUMS`.
- AUR packages `abstract-editor` (source) and `abstract-editor-bin`.

[Unreleased]: https://github.com/fireflylabss/abstract/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/fireflylabss/abstract/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/fireflylabss/abstract/releases/tag/v0.1.0
