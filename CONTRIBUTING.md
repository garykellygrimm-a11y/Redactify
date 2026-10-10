# Contributing to Redactify

Thanks for considering a contribution. This document covers what you need
before opening a first pull request. The full development documentation is in
[docs/](docs/README.md#working-on-redactify).

## Before you start

For anything beyond a small fix, please open an issue first to discuss the
change. Redactify has a specific design philosophy — offline-only, human
review before any redaction is applied, an engine with no I/O opinions — and
it's better to align before you write code than after. See
[Architecture](docs/development/architecture.md).

**No sensitive or proprietary data.** Never include real personal data,
credentials, or content from a private or employer system in code, tests,
fixtures, commit messages, or issues — including as an example of what the
tool should detect. Use synthetic data only.

## Setting up

Requires the [Rust toolchain](https://rustup.rs/), [Node.js](https://nodejs.org/)
24 for the desktop app, and Python 3.11 or newer for the CI helper scripts.

```console
$ git clone https://github.com/garykellygrimm-a11y/Redactify.git
$ cd Redactify
$ cargo build            # engine + CLI
$ cd app && npm ci       # desktop app frontend
```

Run `npm run tauri dev` from `app/` for the desktop app. Cargo commands run
from the repository root; npm commands run from `app/`.

## Branch naming

Branches must match `type/slug`. The slug uses lowercase letters, digits, and
hyphens only, so write a version as `ci/knope-0-23-0`, not `ci/knope-0.23.0`.

| Type | For |
| --- | --- |
| `feat/`, `feature/` | New functionality |
| `fix/`, `bugfix/`, `bug/`, `hotfix/` | Bug repairs |
| `refactor/` | Restructuring with no behavior change |
| `chore/` | Housekeeping — tooling, dependencies, cleanup |
| `docs/` | Documentation only |
| `ci/` | CI/CD workflow changes |

Example: `feat/rule-editor`, `fix/overlap-resolution-tie-break`.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/):
`type(scope): summary`, imperative mood, under ~72 characters, with a body
explaining *why* when the change involves a non-obvious decision.

```text
fix(core): anchor parenthesized area codes in us_phone rule

\b cannot match between a space and '(', so "(816) 555-0142" matched
from the '8', producing a wrong span with an orphaned paren.
```

Common types are `feat`, `fix`, `docs`, `chore`, `ci`, `test`, and
`refactor`, and the scopes are `core`, `cli`, and `app`. The type and scope
decide whether a commit appears in the changelog and triggers a release; see
[Releasing](docs/development/releasing.md#what-counts-as-a-release).

## Before opening a pull request

From the repository root:

```console
$ cargo fmt --all --check
$ cargo clippy --all-targets --locked -- -D warnings
$ cargo test --all --locked
$ cargo audit --deny warnings
$ python scripts/check-tauri-versions.py
```

From `app/`:

```console
$ npm run build
```

These mirror the required CI checks; [CI checks](docs/development/ci.md)
explains each one and how to fix a failure. For frontend changes, also check
the change in both light and dark themes. New detection rules need tests; see
[Testing](docs/development/testing.md#testing-a-detection-rule).

## Pull requests

- Target `main`. All required checks must pass before merging.
- Keep each pull request to one logical change.
- Describe *what* changed and *why*. If it fixes a bug, say what caused it.
- Merge with a merge commit. Squash and rebase merging are disabled, because
  the release tooling reads each commit individually.
- If the change touches `crates/`, `app/src/`, or `app/src-tauri/`, update the
  documentation it affects. For a change with no user-facing effect, add the
  `no-docs-needed` label instead.

## Design decisions

Non-obvious architectural choices are recorded as ADRs in
[docs/adr/](docs/adr/). If you're proposing something that touches how the
manifest represents data, how rules are validated, or a similar foundational
decision, a short ADR alongside the pull request is the right place to
discuss it.

## Reporting bugs and security issues

Use the issue templates for bugs and feature requests. For anything that
might be a security vulnerability, see [SECURITY.md](SECURITY.md) — please
don't open a public issue for those.
