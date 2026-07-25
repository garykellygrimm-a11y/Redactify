# Contributing to Redactify

Thanks for considering a contribution. This document describes the workflow
this project actually uses, not an aspirational one — following it keeps
history and CI meaningful.

## Before you start

For anything beyond a small fix, please open an issue first to discuss the
change. Redactify has a specific design philosophy — offline-only, human
review before any redaction is applied, an engine with no I/O opinions — and
it's better to align before you write code than after.

**No sensitive or proprietary data.** Never include real personal data,
credentials, or content from a private or employer system in code, tests,
fixtures, commit messages, or issues — including as an example of what the
tool should detect. Use synthetic data only (the fixtures under
`tests/fixtures/` are good examples).

## Project layout

```
crates/
├── redactify-core/   # detection, rules, redaction, manifest — pure
│                     # library, no I/O opinions, no knowledge of callers
└── redactify-cli/    # thin frontend over the core
app/
├── src-tauri/        # Rust: Tauri commands, session state, native menus
└── src/              # React/TypeScript: review UI only
```

The core/frontend split is deliberate: detection and redaction logic belongs
in `redactify-core`, never in the CLI or the app. If you're adding a feature
and unsure which side it belongs on — ask, or default to the core.

## Setting up

Requires the [Rust toolchain](https://rustup.rs/) and, for the desktop app,
[Node.js](https://nodejs.org/).

```console
$ git clone https://github.com/garykellygrimm-a11y/Redactify.git
$ cd Redactify
$ cargo build            # engine + CLI
$ cd app && npm install  # desktop app frontend
```

Run `npm run tauri dev` from `app/` for the desktop app, `cargo test` from the
repo root for the test suite.

## Branch naming

Branches are checked in CI against `type/slug`, lowercase, hyphen-separated:

| Type | For |
| --- | --- |
| `feature/` | New functionality |
| `fix/`, `bugfix/`, `hotfix/` | Bug repairs |
| `refactor/` | Restructuring with no behavior change |
| `chore/` | Housekeeping — tooling, dependencies, cleanup |
| `docs/` | Documentation only |
| `ci/` | CI/CD workflow changes |

Example: `feature/rule-editor`, `fix/overlap-resolution-tie-break`.

Automated branches are exempt: `release-please--*` (release-please's
Release PRs) and `dependabot/**` (dependency updates) don't follow
`type/slug` — neither tool supports customizing its branch-naming scheme,
so the naming check excludes them by pattern instead.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/):
`type(scope): summary`, imperative mood, under ~72 characters, with a body
explaining *why* when the change involves a non-obvious decision — a commit
message is the first thing a future maintainer (possibly you) reads when
something breaks.

```
fix(core): anchor parenthesized area codes in us_phone rule

\b cannot match between a space and '(', so "(816) 555-0142" matched
from the '8', producing a wrong span with an orphaned paren.
```

Common types: `feat`, `fix`, `docs`, `chore`, `ci`, `test`, `refactor`.

## Before opening a pull request

```console
$ cargo fmt --all --check
$ cargo clippy --all-targets -- -D warnings
$ cargo test
$ cargo audit --deny warnings   # see .cargo/audit.toml if a new advisory appears
```

For frontend changes, also confirm `npm run tauri dev` runs cleanly and the
change works in both light and dark themes.

Every rule (builtin or in a design doc) should have both a true-positive and a
near-miss negative test — a rule that never fails to match anything hasn't
been tested, it's been assumed.

## Pull requests

- Target `main`. CI (format, lint, test, audit, branch name) must be green.
- Keep PRs scoped to one logical change. A PR that mixes a feature with
  unrelated cleanup is harder to review and harder to revert if something's
  wrong.
- Describe *what* changed and *why*, not just what files moved. If the PR
  resolves a bug, say what caused it — that context outlives the diff.
- Squash-merge is fine for small or mechanical PRs; a multi-commit feature
  with a meaningful history (e.g. "found the bug, fixed the bug" as separate
  commits) can be merged as-is.

## Design decisions

Non-obvious architectural choices are recorded as ADRs in `docs/adr/`. If
you're proposing something that touches how the manifest represents data, how
rules are validated, or a similar foundational decision, a short ADR alongside
the PR is welcome and often the right place to have the discussion.

## Reporting bugs and security issues

Use the issue templates for bugs and feature requests. For anything that
might be a security vulnerability, see [SECURITY.md](SECURITY.md) — please
don't open a public issue for those.
