# Redactify documentation

## Using Redactify

- [Desktop app](user-guide/desktop-app.md) — opening a file, reviewing
  findings, exporting, and keyboard shortcuts
- [Command line](user-guide/cli.md) — `scan`, `verify`, and `batch`
- [Custom rules](user-guide/custom-rules.md) — writing rules files, pattern
  syntax, and the pattern tester
- [The audit manifest](user-guide/audit-manifest.md) — what it records and
  how to verify a redaction

## Reference

- [Detection rules](reference/detection-rules.md) — all 30 builtin rules
- [Manifest format](reference/manifest-format.md) — every manifest field

## Working on Redactify

Start with [CONTRIBUTING](../CONTRIBUTING.md), then:

- [Architecture](development/architecture.md) — how the core and the front
  ends fit together, and the design principles
- [Testing](development/testing.md) — the test suite, rule-test policy, and
  corpus scanning
- [CI checks](development/ci.md) — every check and how to fix a failure
- [Dependencies](development/dependencies.md) — Dependabot, Tauri plugin
  pairs, and toolchains
- [Releasing](development/releasing.md) — how releases happen and how to
  choose a version

## Project

- [Roadmap](roadmap.md) — what has shipped and what is planned
- [Changelog](../CHANGELOG.md) — every release in detail
- [Security policy](../SECURITY.md) — reporting a vulnerability, and known
  advisories in dependencies

## Design decisions

- [ADR 001: Audit manifest content](adr/001-manifest-content.md)
- [ADR 002: User-defined rules](adr/002-user-rules.md)
