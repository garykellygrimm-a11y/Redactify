# Custom rules

Custom rules add patterns for data the builtin rules don't know about, such as
internal hostnames, project codes, or document markings. Both the desktop app
and the CLI read the same rules file format.

## The rules file

A rules file is TOML with one `[[rules]]` table per rule:

```toml
[[rules]]
id = "project_code"
name = "Project Code"
pattern = '(?i)\bPRJ-\d+\b'

[[rules]]
id = "cui_marker"
name = "CUI Banner Marking"
pattern = '(?i)\bCUI//[A-Z]+\b'
```

Every rule needs all three fields:

| Field | Meaning |
| --- | --- |
| `id` | Unique identifier. Appears in output as `[REDACTED:<id>]` and in the manifest. |
| `name` | Human-readable name shown in the app. |
| `pattern` | The regular expression to match. |

Any other key is an error, so a misspelled key such as `[[rule]]` is reported
instead of silently ignored.

Write patterns in single quotes (`'...'`), as above. TOML treats single-quoted
strings literally, so a backslash stays a backslash. In double quotes, every
backslash must be doubled (`"\\bPRJ-\\d+\\b"`).

`examples/custom-rules.toml` is a working example with
`tests/fixtures/custom-sample.log` to try it on.

## Using a rules file

**Desktop app:** **File → Load Rules** (`Ctrl+L`). The open document is
rescanned with the new rules. The app does not yet remember the rules file
between launches, so load it again after restarting.

**CLI:** pass `--rules` to `scan` or `batch`:

```console
$ redactify-cli scan input.log --rules my-rules.toml -o clean.log
```

Add `--no-builtins` to scan with only your rules.

## Replacing a builtin rule

A custom rule with the same `id` as a builtin replaces it. For example, this
limits `ipv4` to addresses in `10.0.0.0/8`:

```toml
[[rules]]
id = "ipv4"
name = "IPv4 (internal only)"
pattern = '\b10\.\d{1,3}\.\d{1,3}\.\d{1,3}\b'
```

The ids of the builtin rules are listed in
[Detection rules](../reference/detection-rules.md).

## Pattern syntax

Patterns use the syntax of Rust's
[`regex`](https://docs.rs/regex/latest/regex/#syntax) crate. Most familiar
regex features work. Two things don't:

- **Look-around** (`(?=...)`, `(?!...)`, `(?<=...)`, `(?<!...)`) is not
  supported. Use `\b` word boundaries to anchor a pattern instead.
- **Backreferences** (`\1`) are not supported.

In exchange, matching always runs in time proportional to the input, so no
pattern can hang a scan. Each compiled pattern is limited to 1 MiB.

Useful pieces:

| Syntax | Meaning |
| --- | --- |
| `(?i)` at the start | Case-insensitive |
| `\b` | Word boundary |
| `\d`, `\w`, `\s` | Digit, word character, whitespace |
| `\S` | Any character except whitespace |

## Errors

A rules file is all or nothing. If any rule is invalid, the whole file is
rejected, nothing is scanned, and the error names the rule. A partly loaded
rule set would produce output you might wrongly believe was fully redacted.
See [ADR 002](../adr/002-user-rules.md) for the reasoning.

| Problem | Error |
| --- | --- |
| Invalid pattern | `invalid pattern in rule 'bad': regex parse error: ...` |
| Look-around in a pattern | `... look-around, including look-ahead and look-behind, is not supported` |
| Two rules with the same `id` | `duplicate rule id 'a' in rules file` |
| A missing field | ``invalid rules file: TOML parse error ... missing field `name` `` |
| An unknown key | ``invalid rules file: TOML parse error ... unknown field `rule`, expected `rules` `` |

## Testing a pattern in the app

The pattern tester was added after version 0.6.2 and ships in the next
release.

The **Rules** tab in the sidebar has a pattern tester. Type a pattern, and
every match in the open document is highlighted as you type, with an exact
count of matches. Highlighting covers the first 200 lines that contain a
match; the count always covers the whole document.

The **Simple / Regex** toggle sets how the input is read:

- **Regex** reads it as a regular expression.
- **Simple** reads it as a wildcard pattern and shows the regular expression it
  generates.

In Simple mode:

| Write | Matches |
| --- | --- |
| `*` | Any run of non-space characters, including none |
| `?` | Exactly one non-space character |
| anything else | Itself, literally |

So `*@example.com` matches whole addresses such as `alice@example.com`
without running across spaces. `*` also matches punctuation, so in
`<alice@example.com>` the match includes the `<`.

The tester previews a pattern; it does not save it. To use a pattern for real,
copy the regular expression it shows into a rules file.
