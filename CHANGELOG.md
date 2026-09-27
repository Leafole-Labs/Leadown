# Changelog

All notable changes to Leadown are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Leadown is a fork of [abstract](https://github.com/fireflylabss/abstract). See
the original project's history for earlier changes.

## [0.1.1] - 2026-09-22

### Changed

- Rebranded from `abstract` to `Leadown` across all user-facing surfaces:
  application name, window title, UI texts, README, documentation, package
  metadata, binary names, and release artifacts.
- Replaced the GPUI-based UI layer with a web-based frontend (HTML/CSS/JS)
  running on Tauri 2.
- Markdown is now converted to sanitized HTML as an intermediate rendering
  representation. Notes remain plain `.md` files on disk.

### Added

- `render` module: Markdown → HTML conversion with XSS-safe sanitization.
- Web frontend: sidebar tree, editor with live preview, search palette,
  theme cycling, first-run tour, i18n (English and Portuguese).
- Tauri 2 backend: filesystem operations, settings, session persistence,
  spaces management.

## [0.1.0] - 2026-09-20

### Added

- Initial fork from `abstract` with GPUI-based rendering.
- Live markdown editing with tree-sitter inline parsing.
- Autosave with atomic writes.
- Spaces (multiple note directories).
- Sidebar tree with inline rename.
- Task lists with clickable checkboxes.
- Syntax-highlighted code blocks.
- Wiki-links with autocomplete and backlinks panel.
- Global search.
- External-edit aware (filesystem watcher).
- Session restore.
- Monochrome light/dark themes.
- English and Portuguese (Brazilian) interface.
- First-run tour.
- Chromeless custom titlebar.
