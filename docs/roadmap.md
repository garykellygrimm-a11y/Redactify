# Roadmap

Redactify's desktop app and CLI share one version number. Each release is
described in detail in [CHANGELOG.md](../CHANGELOG.md).

## Shipped

| Version | Highlights |
| --- | --- |
| M0–M5 | Workspace and CI; detection engine and CLI; audit manifest; custom rules; desktop app with review, export, and themes; installers for Windows, macOS, and Linux |
| 0.5.0 | Virtualized rendering for large documents, recent files, accessibility features and a shortcuts panel, Save and Export, 17 new detection rules |
| 0.6.0 | `redactify-cli verify`, `redactify-cli batch`, 8 new detection rules including checksum-validated IBAN, routing number, Canadian SIN, and Bitcoin address, and capture-group support for rules |
| 0.6.1 | Rules panel in the sidebar; fixed menu shortcuts; fixed false positives in `us_phone`, `credit_card`, and `ipv6` |
| 0.6.2 | Release fix aligning the version across the workspace |

## In progress: v0.7 — Rule authoring in the app

Writing custom rules without leaving the app or knowing regular expressions.
Rules are saved to the same TOML format the CLI reads, so nothing authored in
the app is locked into it.

- [x] Rules panel listing every active rule (0.6.1)
- [x] Live match highlighting while testing a pattern (merged, not yet
      released)
- [x] Simple mode: wildcard patterns such as `*@example.com` translated to a
      regular expression (merged, not yet released)
- [ ] Save a tested pattern as a rule, and keep it between launches
- [ ] Keep existing review decisions when a rule is added mid-review
- [ ] Select-to-suggest: highlight an example in the document and get ranked
      candidate patterns

## Planned

**v0.8 — Signed and self-updating.** Code-signed builds, in-app updates, and
distribution through winget and Homebrew.

**v0.9 — Multiple open files.** Working on more than one document at once.
This is an architectural change: the open document, the review, the view
mode, and the export path all become one instance per document, and Close,
Recent Files, Save, and Export need defined meanings when several files are
open.

**v0.10 — Documents.** PDF and DOCX support, with redaction that removes
content rather than drawing boxes over it.

**v1.0 — Complete and tested.** A completeness bar rather than a feature list:

- correctness checked against a broad corpus of realistic documents, not only
  per-rule tests
- installers exercised through a full review and export on every platform,
  not only built
- `verify` exercised as a real check
- deliberate tests of failure modes: corrupted files, very large files,
  permission errors on export, and malformed custom rules
- documentation that matches what has shipped
- signed builds, carried over from v0.8

## Proposed

Ideas discussed but not yet scheduled into a version.

- **Verify screen.** `verify` in the desktop app: choose an original, an
  output, and a manifest, and see each check pass or fail.
- **Settings screen.** Theme, text size, highlight colors, accessibility
  options such as a high-contrast theme and a colorblind-safe palette, and the
  rules file to load at startup. Customizable key bindings may follow.
- **Navigation.** Moving between the document, Verify, and Settings. To be
  decided in an ADR before the first new screen is built.

## Open questions

- **Updates and offline use.** v0.8's in-app updates require contacting a
  server, and Redactify promises to run fully offline. How the two coexist,
  for example an opt-in update check or importing a downloaded update, needs
  a decision before v0.8 begins.
