# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0](https://github.com/garykellygrimm-a11y/Redactify/compare/app-v0.5.0...app-v0.6.0) (2026-07-26)


### Added

* **app:** add recent files menu ([#54](https://github.com/garykellygrimm-a11y/Redactify/issues/54)) ([005c9f6](https://github.com/garykellygrimm-a11y/Redactify/commit/005c9f6d3aa3d81a80ce2d848a43460110dbf146))
* **app:** add Save (Ctrl+S) and Export (Ctrl+E) ([#57](https://github.com/garykellygrimm-a11y/Redactify/issues/57)) ([1baa057](https://github.com/garykellygrimm-a11y/Redactify/commit/1baa057735391e5451ebb5c6f87923188115ca83))
* **app:** before/after preview toggle (Ctrl+D, View menu) ([b68bc2f](https://github.com/garykellygrimm-a11y/Redactify/commit/b68bc2f21e2c429b689c69d45d5df5496b29c2d9))
* **app:** before/after preview toggle (Ctrl+D, View menu) ([bbc975d](https://github.com/garykellygrimm-a11y/Redactify/commit/bbc975d2a46b7b0c35f546f078e592489a1764a2))
* **app:** brand the NSIS installer ([dd74b18](https://github.com/garykellygrimm-a11y/Redactify/commit/dd74b184d8682274d768a29943fbfd51d67b0481))
* **app:** design token system, themes, and bundled fonts ([35484f2](https://github.com/garykellygrimm-a11y/Redactify/commit/35484f2e3e036033f42de2c4b24eac60a85bff99))
* **app:** design token system, themes, and bundled fonts ([9e5dbdb](https://github.com/garykellygrimm-a11y/Redactify/commit/9e5dbdbc09c8aa9f81363fe78f99bbaf8c1c79c6))
* **app:** export — redacted file, disposition manifest, success view ([2cbbe41](https://github.com/garykellygrimm-a11y/Redactify/commit/2cbbe4185e1a99aaaae715facab252fd3d51b112))
* **app:** expose scan_text command backed by redactify-core ([08857bd](https://github.com/garykellygrimm-a11y/Redactify/commit/08857bd976e1c8cfdbc82e79bb0e0d34008acc1f))
* **app:** file opening via Browse and native drag-and-drop ([0aeacb2](https://github.com/garykellygrimm-a11y/Redactify/commit/0aeacb2bcd69c8590ae99e27b9f9fccf6e8f13be))
* **app:** inline finding highlights with rule-hue palette and jump-to ([c6e4fea](https://github.com/garykellygrimm-a11y/Redactify/commit/c6e4fead09b86ea1ce0e90fc95eafd7b7dbef328))
* **app:** native menu bar — File (Open, Close Document, Exit), View ([2f01329](https://github.com/garykellygrimm-a11y/Redactify/commit/2f0132961e909324da801f392f417628535e3ad4))
* **app:** Redactify application icon ([74dc4b6](https://github.com/garykellygrimm-a11y/Redactify/commit/74dc4b6c9b5efa3b5c4c686ec049a54307a4532d))
* **app:** window persistence, honest titles, discard confirmations ([65a7b3f](https://github.com/garykellygrimm-a11y/Redactify/commit/65a7b3f3aee2d6fe8d17733d96d9f578f2d153f4))


### Fixed

* **ci:** restore release-plz's release memory; lockstep versions ([9c65fb7](https://github.com/garykellygrimm-a11y/Redactify/commit/9c65fb7f2fc72802a1f8398a28a794f6742c20bb))
* **ci:** restore release-plz's release memory; lockstep versions ([c632d05](https://github.com/garykellygrimm-a11y/Redactify/commit/c632d05cfe5292a4bc4d6b38ad6af961e0edc408))
* **ci:** scope release-please to app/src-tauri; sync the rest via ([#76](https://github.com/garykellygrimm-a11y/Redactify/issues/76)) ([cff097f](https://github.com/garykellygrimm-a11y/Redactify/commit/cff097f43a40dc385283d70d62770be3eaaea01d))

## [0.5.0] - 2026-07-25

### Added

- Virtualized document rendering — files with tens of thousands of lines
  now scan and scroll smoothly, since only the visible rows are ever
  mounted
- Recent Files menu (File > Open Recent), capped at 5 entries, with
  self-healing for files that have moved or been deleted
- Adjustable text size, visible keyboard-focus indicators, reduced-motion
  support, and a keyboard shortcuts help panel (`?` or the toolbar button)
- Save (`Ctrl+S`, reuses the last export destination) alongside the
  existing Export (`Ctrl+E`, always prompts)
- A visible Before/After toggle in the toolbar; pending findings are now
  visually distinct from rejected ones in the After preview, and the
  preview banner shows a live pending-findings count
- 17 new detection rules — GCP API key & OAuth client ID, Oracle Cloud
  Identifier, Azure SAS token, a generic PEM private-key block, Stripe,
  DigitalOcean, SendGrid, GitHub, Slack, HashiCorp Vault, IPv6, OpenAI,
  Anthropic, npm, and Twilio — plus a Luhn-validated credit/debit card
  rule. 22 builtin rules in total, up from 5
- Automated version bumps and changelog generation via release-please

### Changed

- Undo moved from `u` to the more universally recognized `Ctrl+Z`;
  `j`/`k` removed in favor of arrow-key-only navigation; whole-rule
  accept/reject is now labeled `Shift+A` / `Shift+R`

### Fixed

- Theme preference is now actually applied on launch — it was previously
  computed but never used, so the app silently reset to light every time
  regardless of the saved preference

## [0.4.4] - 2026-07-22

First public release — downloadable installers and binaries.

### Added

- Windows (`.exe` / `.msi`), macOS (`.dmg`), and Linux (`.deb` / `.AppImage`)
  installers, built for all three platforms on tag push
- Standalone CLI binaries for Windows, macOS (arm64), and Linux, each with a
  SHA-256 checksum. The Windows build links the C runtime statically, so it
  runs on a bare host with no dependencies — including air-gapped machines
- Application icon and branded installer: wizard artwork, MIT license page,
  and per-user installation (no administrator prompt)

### Changed

- The window opens at 1100x720 with an 800x560 minimum size
- The application version is now inherited from `Cargo.toml` rather than
  declared twice

Versions 0.4.1 through 0.4.3 were internal bumps produced while evaluating
release automation and were never released.

## [0.4.0] - 2026-07-20

Desktop application.

### Added

- Tauri desktop app with a three-zone review interface: findings sidebar,
  highlighted document, and verdict strip
- Keyboard-driven review — `j`/`k` to walk findings, `a`/`r` to accept or
  reject, `A`/`R` for an entire rule, `u` to undo
- Accepted findings render in place as `[REDACTED:rule]` tokens, making the
  document a live preview of the output
- Export gate: findings must all be decided before a file can be written
- Audit manifests now record each finding's disposition — accepted (redacted)
  or rejected (a reviewer saw it and declined) — alongside an applied count
- Custom rules loading, document search, before/after output preview, light
  and dark themes, window-state persistence, and confirmation before an
  in-progress review is discarded
- Native menus: open, load rules, close document, preview and theme toggles

### Fixed

- Review of very large files: memoized components, debounced search, and
  imperative match stepping keep a 30,001-line document responsive. Scanning
  that file (30,000 findings) takes 165 ms

## [0.3.0] - 2026-07-17

User-defined rules.

### Added

- Detection rules can be supplied in TOML and merged over the builtins, with
  same-id rules overriding them; `--rules` and `--no-builtins` in the CLI
- Fail-fast validation: any invalid pattern or duplicate id aborts the entire
  load with the offending rule named, rather than scanning with a partial
  rule set
- A compile-size cap on user-supplied patterns

## [0.2.0] - 2026-07-16

Audit manifest.

### Added

- JSON audit manifest recording the active rules, every finding's rule and
  byte span, and SHA-256 hashes of both the complete source and the complete
  output — verifiable with any standard checksum tool
- `clap`-based CLI with `--output` and `--manifest`
- Typed, `Result`-based errors throughout the core library

## [0.1.0] - 2026-07-16

Detection engine.

### Added

- Detection engine with five builtin rules: email addresses, IPv4 addresses,
  US Social Security numbers, US phone numbers, and AWS access key IDs
- Overlap resolution — findings are always returned sorted and
  non-overlapping
- Redaction with rule-tagged replacement tokens
- CLI that writes sanitized text to stdout and a findings summary to stderr,
  so shell redirection captures clean output only
