# CI checks

Every workflow lives in `.github/workflows/`. This page lists each check by
the name shown on a pull request, what it verifies, and how to fix a failure.

## Required checks

A pull request into `main` cannot merge until these pass:

- Format, Lint, Test
- Tauri crate/npm versions
- Frontend type-check and build
- cargo audit
- docs-updated
- Enforce Naming Policy

CodeQL also runs but is not required.

## Format, Lint, Test

**Workflow:** `ci.yml`. Runs on pull requests and pushes to `main`.

Runs, from the repository root:

```console
$ cargo fmt --all --check
$ cargo clippy --all-targets --locked -- -D warnings
$ cargo test --all --locked
```

**On failure:**

- *Formatting:* run `cargo fmt --all` and commit the result.
- *Clippy:* every warning is an error. Fix the code clippy points to; do not
  add `allow` attributes to silence it without a reason in the commit.
- *"the lock file needs to be updated":* `--locked` stops Cargo from changing
  `Cargo.lock`. A `Cargo.toml` changed without its lockfile. Run
  `cargo build` locally and commit the updated `Cargo.lock`.

## Tauri crate/npm versions

**Workflow:** `ci.yml`. Runs on pull requests and pushes to `main`.

Runs `scripts/check-tauri-versions.py`, which compares every Tauri crate in
`Cargo.lock` with its npm package in `app/package-lock.json` and fails if any
pair differs in major.minor version. `tauri build` refuses to build a
mismatched pair, so this catches it before a release does.

**On failure:** the output marks each pair `ok` or `MISMATCH`. Update the
lagging side as described in
[Dependencies](dependencies.md#tauri-plugins). The script needs Python 3.11 or
newer to run locally.

## Frontend type-check and build

**Workflow:** `ci.yml`. Runs on pull requests and pushes to `main`.

Runs, from `app/` on Node 24:

```console
$ npm ci
$ npm run build
```

`npm run build` runs `tsc` to type-check the frontend, then `vite build`.

**On failure:**

- *`npm ci` fails:* `package.json` and `package-lock.json` disagree. Run
  `npm install` in `app/` and commit the updated lockfile.
- *`tsc` fails:* fix the reported type errors. Dependency updates to React,
  Vite, or TypeScript are the usual cause.

## cargo audit

**Workflow:** `audit.yml`. Runs on pull requests, pushes to `main`, every
Monday at 13:00 UTC, and manually.

Runs `cargo audit --deny warnings`, which checks every dependency against the
RustSec advisory database. Informational advisories, such as unmaintained
crates, fail the build too.

Advisories are published against code that has not changed, so this check can
fail on a pull request that touched nothing related. The scheduled run exists
to catch those early.

**On failure:** read the advisory, then either:

- update the affected dependency to a fixed version, or
- if no fix exists or the advisory does not apply, add it to the `ignore` list
  in `.cargo/audit.toml` with the reason it is ignored and the condition that
  should retire the entry, and add the matching row to the "Known advisories
  in dependencies" table in `SECURITY.md`.

## docs-updated

**Workflow:** `docs-check.yml`. Runs on pull request events, including when a
label is added or removed.

Fails if the pull request changes `crates/`, `app/src/`, or `app/src-tauri/`
without also changing `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, or a file
under `docs/`. Cargo and npm manifests and lockfiles do not count as code, so
dependency and release pull requests pass on their own.

**On failure:** update the documentation the change affects. If the change
has no user-facing effect, such as an internal refactor, add the
`no-docs-needed` label instead; the check then skips, which counts as passing.

## Enforce Naming Policy

**Workflow:** `branch-name-check.yml`. Runs on every push to a branch other
than `main`. Skipped for Dependabot branches.

Fails unless the branch name matches `type/slug`: a type from the list in
[CONTRIBUTING](../../CONTRIBUTING.md#branch-naming), a slash, then lowercase
letters, digits, and hyphens.

**On failure:** rename the branch and push it again:

```console
$ git branch -m new-type/new-slug
$ git push -u origin new-type/new-slug
```

Then delete the old remote branch and open the pull request from the new one.

## CodeQL

**Workflow:** `codeql.yml`. Runs on pull requests into `main`, pushes to
`main`, and every Thursday at 04:30 UTC.

Two jobs:

- **CodeQL Logic Scan** analyzes the Rust and TypeScript code with the
  `security-extended` and `security-and-quality` query suites.
- **Frontend Quality Scan** runs HTMLHint and Stylelint over the HTML and CSS.

Findings appear in the repository's Security tab under code scanning.

## Release workflows

`Prepare Release` and `Release` run on pushes to `main`, not on pull
requests, and can also be started manually from the Actions tab. They are
described in [Releasing](releasing.md).
