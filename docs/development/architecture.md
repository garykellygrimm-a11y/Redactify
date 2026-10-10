# Architecture

Redactify is a Cargo workspace with one detection engine and two front ends.

```text
crates/
├── redactify-core/   # detection, rules, redaction, manifest
└── redactify-cli/    # command-line front end over the core
app/
├── src-tauri/        # Rust: Tauri commands, session state, native menus
└── src/              # React/TypeScript: review UI only
```

## The core

`redactify-core` holds everything that decides what is sensitive and what the
output looks like:

- **Detection.** `detect()` runs every rule over the text and returns findings
  sorted by start offset. Overlaps are resolved in one pass: the earliest
  start wins, and on a tie the longest match wins.
- **Rules.** The builtin rules, custom rules loaded from TOML, validation, and
  glob-to-regex translation. A custom rule with the same id as a builtin
  replaces it.
- **Redaction.** `redact()` replaces each accepted finding with
  `[REDACTED:<rule_id>]`.
- **The manifest.** Building the audit manifest and verifying one.

The core never opens documents or writes output; the front ends own all of
that. The one exception is `load_rules_file()`, which reads a rules file from
a path it is given.

Detection and redaction logic belongs in the core, never in a front end. When
it is unclear which side a change belongs on, it belongs in the core.

## The CLI

`redactify-cli` reads files, calls the core, and writes the redacted output
and manifest. It has no review step, so every finding in a CLI manifest is
recorded as accepted.

## The desktop app

The desktop app is a Tauri application. Its Rust side, `app/src-tauri`, holds
the session state and exposes commands to the frontend: `open_file`,
`load_rules`, `list_rules`, `preview_pattern`, `close_document`, and
`export`. Its React side, `app/src`, is the review interface.

A document moves through the app like this:

1. `open_file` reads the file, runs the core's `detect()`, and keeps the text
   and findings in Rust-side state.
2. Rust splits each line into render-ready segments and sends them, with the
   findings, to the frontend.
3. The reviewer accepts or rejects findings. The frontend records each
   decision in its review log.
4. `export` receives only the indices of the accepted findings. Rust redacts
   its own copy of the text and writes the output and the manifest.

## Design principles

**The frontend never does offset math.** Findings are UTF-8 byte offsets;
JavaScript strings are UTF-16. Rust splits every line into render-ready spans
before sending it, so highlights cannot be misaligned on non-ASCII text.

**Review state is an event log.** Decisions are appended to a log, and the
current state is derived by replaying it. Undo removes the last event. A
rule-wide sweep is recorded as one event, so a single undo reverses it. Bulk
actions only change pending findings and never overwrite an individual
decision.

**The engine owns the document.** The open document lives in Rust-side state.
The frontend holds display segments and decisions, and export sends only
decisions back, never document content.

**Matching favors recall.** Rules are written to catch too much rather than
too little. False positives are acceptable because a person reviews every
finding before anything is redacted.

**Fully offline.** Nothing in the core, the CLI, or the app makes network
requests. Redactify is built to run in air-gapped and restricted
environments.

## Known limitations

**Large files.** `open_file` reads the whole file into memory, the segments
are built as separate owned strings, and the complete scan result crosses the
Tauri IPC bridge as JSON. Memory use is a multiple of the file size. Files far
larger than typical logs have not been tested in the desktop app.

## Design decisions

Foundational decisions are recorded as architecture decision records:

- [ADR 001: Audit manifest content](../adr/001-manifest-content.md)
- [ADR 002: User-defined rules](../adr/002-user-rules.md)
