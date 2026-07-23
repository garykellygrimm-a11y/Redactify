# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately through GitHub's
[private vulnerability reporting](https://github.com/garykellygrimm-a11y/Redactify/security/advisories/new)
(Security tab → Report a vulnerability) rather than opening a public issue.

This is a single-maintainer project. Reports are acknowledged on a best-effort
basis, typically within a week. Please include the version, platform, and steps
to reproduce.

## Supported versions

| Version | Supported |
| --- | --- |
| 0.4.x | Yes |
| < 0.4 | No |

Fixes land in the latest release only.

## What Redactify guarantees — and what it does not

**Guaranteed:**

- **No network access.** The application and CLI perform no network calls and
  collect no telemetry. Files never leave the machine they are processed on.
- **Audit manifests contain no sensitive content.** Manifests record rule ids,
  byte spans, lengths, and dispositions — never matched text and never
  per-finding hashes, which would be reversible for low-entropy values such as
  SSNs and phone numbers. See
  [ADR 001](docs/adr/001-manifest-content.md).
- **Rule files are validated atomically.** An invalid or duplicate user rule
  aborts the entire load rather than scanning with a partial rule set, which
  would produce output the user wrongly believes is fully redacted. See
  [ADR 002](docs/adr/002-user-rules.md).
- **Hostile rule patterns cannot hang the scanner.** Rust's regex engine has no
  backtracking and matches in linear time, so catastrophic backtracking (ReDoS)
  is impossible by construction. A compile-size cap bounds the remaining memory
  vector.

**Not guaranteed:**

- **Detection completeness.** Redactify finds what its rules describe. It is a
  pattern matcher, not an oracle: sensitive data in an unexpected format will
  not be detected, and no rule set should be assumed exhaustive. Human review
  of every finding — and of the document as a whole — is the control that makes
  redaction trustworthy. Treat any claim of automatic, complete redaction with
  suspicion, including from this tool.
- **Signed builds.** Release artifacts are currently unsigned. Verify the
  SHA-256 checksums published with each release before running a download.

## Known advisories in dependencies

Every advisory currently reported against this project is inherited
transitively from Tauri. None originate in a dependency this project selected
directly — `regex`, `serde`, `sha2`, `thiserror`, `chrono`, `toml`, and `clap`
are all clean.

Most come from the **GTK3 stack that Tauri v2 uses on Linux**
(`tauri` -> `wry` -> `webkit2gtk` -> `gtk` -> `glib`). These crates are not
compiled at all in the Windows or macOS builds.

| Advisory | Crate(s) | Assessment |
| --- | --- | --- |
| RUSTSEC-2024-0429 | `glib` 0.18.5 | Unsoundness in `VariantStrIter`'s iterator implementations. Redactify never iterates GVariant string arrays. No fix exists in the 0.18 line; the patch requires glib 0.20, which the GTK3 binding generation cannot use. |
| RUSTSEC-2024-0411 through 0420 | `gtk`, `gdk`, `atk`, `gdkx11`, `gdkwayland-sys`, `gtk3-macros`, and their `-sys` crates | GTK3 bindings are unmaintained. Informational; no known exploit. |
| RUSTSEC-2024-0370 | `proc-macro-error` | Unmaintained. Reached via `glib-macros` and `gtk3-macros`. |
| RUSTSEC-2020-0053 | `dirs` | Unmaintained. Reached via `tauri`, `tauri-build`, `tray-icon`, and `wry`. |
| RUSTSEC-2025-0075, 0080, 0081, 0098, 0100 | `unic-*` | Unmaintained. Reached via `urlpattern`. |

These advisories also appear in Tauri's own release audits, which is the
clearest evidence that they are not resolvable downstream. Tauri tracks the fix
as a migration to gtk4-rs and webkit6
([tauri#12561](https://github.com/tauri-apps/tauri/issues/12561),
[tauri#12563](https://github.com/tauri-apps/tauri/issues/12563)); the work is in
progress upstream and is expected in a major Tauri release.

**Our commitment:** when a Tauri release ships the migrated stack, upgrading to
it is treated as a security task, not a routine dependency bump. Until then, CI
runs `cargo audit --deny warnings` on every change and weekly on a schedule,
with exactly these advisories allowlisted and justified in
[`.cargo/audit.toml`](.cargo/audit.toml). Any new finding — vulnerability or
informational — fails the build.

## Scope

In scope: the detection engine, redaction correctness, manifest integrity, rule
file handling, and anything that could cause sensitive content to survive into
an exported file or leak into a manifest.

Out of scope: the dependency advisories listed above, and warnings from
operating systems about unsigned installers.
