## What

<!-- What does this PR change, in a sentence or two? -->

## Why

<!-- Why is this change needed? Link an issue if one exists. -->

## How to verify

<!-- Commands or steps a reviewer can run to confirm this works. -->

## Checklist

- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --all-targets --locked -- -D warnings` passes
- [ ] `cargo test --all --locked` passes
- [ ] `cargo audit --deny warnings` passes (or a new advisory is documented in `.cargo/audit.toml` and `SECURITY.md`)
- [ ] `python scripts/check-tauri-versions.py` passes
- [ ] `npm run build` passes in `app/`
- [ ] New behavior has tests — a true positive and, where applicable, a near-miss negative
- [ ] Frontend changes checked in both light and dark themes
- [ ] Documentation updated, or the `no-docs-needed` label applied
- [ ] No real personal data, credentials, or non-public content in code, tests, or commit messages
- [ ] Non-obvious design decisions are explained in a commit message or, if foundational, an ADR

## Notable

<!-- Anything a reviewer should know: tradeoffs made, alternatives considered
     and rejected, follow-up work intentionally left out of scope. -->
