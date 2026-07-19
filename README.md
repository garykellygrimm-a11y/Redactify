# Redactify

[![CI](https://github.com/garykellygrimm-a11y/Redactify/actions/workflows/ci.yml/badge.svg)](https://github.com/garykellygrimm-a11y/Redactify/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Redaction you can prove.** Redactify scans text for sensitive data — PII
and secrets — and produces sanitized output. It runs entirely offline: no
cloud, no ML model downloads, no telemetry. Your files never leave your
machine.

Built in Rust for speed and memory safety, with a core engine designed to
serve multiple frontends: a CLI today, a desktop app and audit manifests on
the roadmap.

## Why another redaction tool?

Existing tools are fire-and-forget: text in, redacted text out, hope for the
best. Redactify is being built around two ideas the ecosystem underserves:

1. **Human-in-the-loop review.** Automated detection produces false
   positives. The endgame is a review workflow — accept or reject each
   finding before anything is applied — rather than blind substitution.
2. **Provable redaction.** A planned audit manifest records what was
   redacted, where, and by which rule, so compliance workflows can
   demonstrate that sanitization actually happened.

Fully offline, deterministic operation is a design constraint, not an
afterthought — Redactify is meant to work in air-gapped and restricted
environments.

## Current status

Early development. Working today:

- Detection engine with five builtin rules: email addresses, IPv4
  addresses, US Social Security numbers, US phone numbers, and AWS access
  key IDs
- Overlap resolution — findings are always sorted and non-overlapping
  (earliest match wins; longest wins ties)
- Redaction with rule-tagged tokens (`[REDACTED:email]`)
- CLI that reads a file, writes sanitized text to stdout and a findings
  summary to stderr — so shell redirection captures clean output only

## Demo

```console
$ redactify sample.log > clean.log
6 findings: 1 aws_access_key, 1 email, 2 ipv4, 1 ssn, 1 us_phone

$ head -3 clean.log
2026-07-16T09:14:02Z INFO  user [REDACTED:email] logged in from [REDACTED:ipv4]
2026-07-16T09:14:05Z DEBUG session token issued, callback to [REDACTED:ipv4]
2026-07-16T09:15:11Z WARN  payment update for SSN [REDACTED:ssn] requested
```

## Building from source

Requires the [Rust toolchain](https://rustup.rs/).

```console
$ git clone https://github.com/garykellygrimm-a11y/Redactify.git
$ cd Redactify
$ cargo build --release
$ ./target/release/redactify-cli tests/fixtures/sample.log
```

Run the test suite with `cargo test`.

## Architecture

Cargo workspace with a strict core/frontend split:
```
crates/
├── redactify-core/   # detection engine, rules, redaction — pure library,
│                     # no I/O opinions, no knowledge of its callers
└── redactify-cli/    # thin frontend over the core
```
`redactify-core` exposes a small API — `detect(text, rules)` returns
sorted, non-overlapping findings; `redact(text, findings)` rebuilds the
text with findings replaced. Every future frontend (desktop app, additional
tooling) calls the same engine, so detection behavior can never drift
between interfaces.

Design decisions worth noting:

- **Findings carry byte offsets into immutable input.** `redact()` builds a
  new string in a single forward pass rather than mutating in place, so
  offsets can never be invalidated by earlier replacements.
- **Liberal matching, by policy.** The SSN rule matches on shape and does
  not validate SSA area-number rules. False positives are acceptable
  because the product thesis is human review, not blind trust.
- **Builtin patterns fail loudly.** Rule compilation panics on invalid
  patterns — a broken builtin should explode in tests, not silently skip.

## Roadmap

- [x] **M0** — Workspace, CI (fmt / clippy / test), branch policy
- [x] **M1** — Detection engine, builtin rules, overlap resolution, CLI
- [x] **M2** — Audit manifest (JSON), `Result`-based errors, `clap` CLI
- [x] **M3** — User-defined rules (config file)
- [x] **M4** — Desktop app (Tauri): interactive review UI — accept/reject
      findings before applying
- [ ] **M5** — Installers and binary releases (Windows / macOS / Linux)

## License

MIT — see [LICENSE](LICENSE).

Cargo workspace with a strict core/frontend split:
