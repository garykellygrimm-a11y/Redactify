# Testing

## Running the tests

From the repository root:

```console
$ cargo test --all --locked
```

All automated tests currently live in `redactify-core`:

| File | Covers |
| --- | --- |
| `crates/redactify-core/src/lib.rs` | Each builtin detection rule, overlap resolution, and redaction |
| `crates/redactify-core/src/rules.rs` | Loading custom rule files, validation, overrides, and glob-to-regex translation |
| `crates/redactify-core/src/manifest.rs` | Manifest contents, JSON round-trips, and `verify` |

`redactify-cli` and the desktop app have no automated tests yet. Changes to
either are checked by building them and exercising them by hand.

## Testing a detection rule

Every rule needs at least two tests:

- **A true positive:** realistic input the rule must match, asserting the
  rule id and, where it matters, the exact matched text.
- **A near miss:** input that looks almost right but must not match, such as
  a wrong prefix, a failed checksum, or a value spanning two log fields.

A rule that has never been shown to reject anything hasn't been tested. For
checksum-validated rules, the near miss should have the right shape and a
wrong checksum, which proves the checksum is actually checked.

The existing tests in `lib.rs` follow this pattern and are the best model for
a new one.

## Test data

Use synthetic data only. Never put real personal data, credentials, or
content from a private or employer system in a test, fixture, or commit
message, even as an example of something the tool should detect. Use
documented example values where they exist, such as `AKIAIOSFODNN7EXAMPLE`
for an AWS key or `example.com` domains for email.

`tests/fixtures/` holds sample inputs for trying the CLI and the desktop app
by hand. No automated test reads them.

| File | Use |
| --- | --- |
| `sample.log` | A short log with findings for several common rules |
| `custom-sample.log` | Input for trying custom rules, with `examples/custom-rules.toml` |
| `big.log` | 30,000 lines, for checking performance and scrolling |

## Corpus scanning

Unit tests show that a rule matches and rejects the cases someone thought of.
They don't show what the rule does to a real log file, where a rule can be
technically correct and still fire on every timestamp. The corpus-scan
harness answers that question.

It runs every builtin rule across a directory of files and reports where each
rule fires:

```console
$ cargo run --release -p redactify-core --example corpus_scan -- <path>
$ cargo run --release -p redactify-core --example corpus_scan -- <path> --samples 5 --ext log,txt
```

- `--samples N` sets how many example matches to show per rule (default 3).
- `--ext` sets which file extensions to scan (default `log,txt`).
- `--all` scans every file regardless of extension.

Always use `--release`; the regex engine is much slower in a debug build. The
output is deterministic, so run it before and after a rule change and diff the
two reports to see whether the change helped.

The corpus itself is never committed. Public log collections such as
[loghub](https://github.com/logpai/loghub) work well. Corpus scanning against
loghub is how the false positives in the `us_phone`, `credit_card`, and `ipv6`
rules were found and fixed in 0.6.1.

## Before a pull request

Run the full set of checks listed in
[CONTRIBUTING](../../CONTRIBUTING.md#before-opening-a-pull-request). Each one
maps to a CI check described in [CI checks](ci.md).
