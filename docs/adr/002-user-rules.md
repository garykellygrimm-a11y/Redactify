# ADR 002: User-defined rules — TOML, fail-fast validation, override-allowed

Status: accepted · Date: 2026-07-17

## Context

Teams need to add organization-specific patterns (CUI markings, internal
hostnames, project identifiers) without modifying source. Accepting
patterns from configuration raises three questions: what format, what
happens on invalid input, and what a hostile pattern can do.

## Decisions

**Format: TOML.** Rust-ecosystem convention (users already read
Cargo.toml), first-class comments so compliance teams can annotate why a
pattern exists, and none of YAML's implicit-typing traps. Schema is a
list of `[[rules]]` tables with `id`, `name`, `pattern`.

**Validation: fail fast, fail loud.** Any invalid pattern or duplicate
id aborts the entire load; nothing is scanned. Skipping a bad rule and
proceeding would produce output the user believes is fully redacted and
is not — a false sense of sanitization is the one failure mode this
tool must never have. This mirrors, deliberately, the opposite policy
for builtins: builtins panic on compile failure (our bug, caught in
tests); user rules return typed errors naming the offending rule id
(their input, handled gracefully).

**Collision policy: user rules override builtins.** A user rule whose
id matches a builtin replaces it, so teams can tighten or swap
defaults. `--no-builtins` supports rules-file-only operation.
Duplicate ids within one file are an error.

## Security posture

Rust's regex crate has no backtracking engine; matching is guaranteed
linear-time, so catastrophic backtracking (ReDoS) is impossible by
construction. The remaining resource a hostile pattern can attack is
memory at compile time, capped via `RegexBuilder::size_limit` (1 MiB)
on user patterns. Builtins are exempt from the cap: we author them and
tests compile them.

## Consequences

- A rules file is rejected atomically; users fix errors before any scan.
- Overriding a builtin id silently changes default behavior — accepted,
  as the manifest's `rules_applied` records exactly what ran.
- Lookahead/lookbehind are unavailable to rule authors (engine
  limitation); patterns lean on word boundaries, as builtins do.
