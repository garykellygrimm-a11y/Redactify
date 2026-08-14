# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Redactify ships as one product: the desktop app and the CLI share a version
number and a release. Entries below are written by Knope from conventional
commits and changesets.

## 0.6.3 (2026-08-14)

### Fixes

- align redactify-core version with the rest of the workspace

## 0.6.2 (2026-08-14)

### Fixes

- align redactify-core version with the rest of the workspace

## 0.6.1 (2026-08-03)

### Added

- Rules panel in the sidebar, listing every active rule with its origin,
  pattern, and whether it is checksum-validated

### Fixed

- Ctrl+O, Ctrl+L, Ctrl+W and Ctrl+T did nothing — the native menu
  accelerators never fired, and only shortcuts that also had a JS binding
  worked
- Three detection rules produced 43% of all findings on real log files and
  none of them were real: `us_phone` matched any bare ten-digit run,
  `credit_card` matched across whitespace-separated log fields, and `ipv6`
  matched C++/Rust scope resolution and hardware addresses

## 0.6.0 (2026-07-30)

### Added

- `redactify verify`, which proves a manifest's account of a redaction by
  regenerating the output from the original plus the recorded decisions and
  comparing byte-for-byte
- `redactify batch` for scanning many files and directories in parallel
- Eight more detection rules (30 builtins, up from 22), four of them
  checksum-validated: IBAN, US bank routing number, Canadian SIN, and
  Bitcoin address
- Capture-group support on `Rule`, and a database-connection-string rule
  built on it that flags only the embedded credential

### Changed

- **Breaking:** the CLI now requires an explicit subcommand.
  `redactify file.txt -o out.txt` is now `redactify scan file.txt -o out.txt`

### Fixed

- Canadian SIN checksum bug and a wrong test assertion

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
