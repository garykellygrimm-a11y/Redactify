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
rescanned with the new rules. The chosen file becomes the app's active rules
file and is loaded again each time the app starts; see
[The active rules file](#the-active-rules-file).

**CLI:** pass `--rules` to `scan` or `batch`:

```console
$ redactify-cli scan input.log --rules my-rules.toml -o clean.log
```

Add `--no-builtins` to scan with only your rules.

## The active rules file

The desktop app has at most one active rules file. Its rules are merged over
the builtin rules, and its name and the number of active rules are shown in
the top bar, as `rules: <file name> · <count>`.

The app remembers the active file's path in `settings.json` in its config
folder and loads the file again at startup. The config folder is Tauri's
standard app config directory, named after the app's identifier
`com.garykellygrimm.redactify`:

| Platform | Config folder |
| --- | --- |
| Windows | `%APPDATA%\com.garykellygrimm.redactify` |
| macOS | `~/Library/Application Support/com.garykellygrimm.redactify` |
| Linux | `$XDG_CONFIG_HOME/com.garykellygrimm.redactify`, usually `~/.config/com.garykellygrimm.redactify` |

Loading a different file with **File → Load Rules** makes that file active
and remembers it instead. If the app cannot write `settings.json`, the rules
still apply for this session, and the banner says
`The rules are active, but will not be remembered next time:` followed by the
reason.

If the remembered file is missing or invalid at startup, the app does not
skip it silently. The banner shows
`Your rules file did not load, so only builtin rules are active.` followed by
the error, the top bar shows `rules: builtin only`, and the app scans with
builtin rules only. The path stays
remembered, so the same error appears at each launch until the file is fixed
or another file is loaded. Fix the file, then load it again with
**File → Load Rules**.

The CLI never reads `settings.json` or the app's config folder. It uses only
the file passed with `--rules`, so to use the app's rules in the CLI, pass the
same file:

```powershell
redactify-cli scan input.log --rules "$env:APPDATA\com.garykellygrimm.redactify\rules.toml" -o clean.log
```

On macOS and Linux, `settings.json` and a `rules.toml` created by the app are
readable only by your user (mode `0600`), and the app sets its config folder
to mode `0700`, including when the folder already exists. The app does not
encrypt rules
files. Custom rules can reveal what your organization treats as sensitive, so
protect them as you protect the documents they apply to: use full-disk
encryption, such as BitLocker, FileVault, or dm-crypt, on machines that hold
sensitive documents. See [ADR 004](../adr/004-rule-persistence.md) for the
reasoning.

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
rejected, nothing is scanned, and the error names the file and the problem. A
partly loaded rule set would produce output you might wrongly believe was fully
redacted. See [ADR 002](../adr/002-user-rules.md) for the reasoning.

Every error starts with the path of the rules file, such as
`rules file 'my-rules.toml': `, followed by:

| Problem | Error |
| --- | --- |
| Invalid pattern | `invalid pattern in rule 'bad': regex parse error: ...` |
| Look-around in a pattern | `... look-around, including look-ahead and look-behind, is not supported` |
| Two rules with the same `id` | `duplicate rule id 'a' in rules file` |
| A missing field | ``TOML parse error ... missing field `name` `` |
| An unknown key | ``TOML parse error ... unknown field `rule`, expected `rules` `` |

Saving a rule from the app can also report
``cannot add a rule: `rules` must be a list of [[rules]] tables``, after the
same `rules file '...': ` prefix, or, if the file cannot be written,
`could not write rules file '...': ` followed by the system's reason.

## Testing a pattern in the app

The pattern tester was added in version 0.7.0.

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

## Saving a tested pattern as a rule

Once the tester holds a valid pattern, fields for a rule id and a name appear
below it, with a **Save as rule** button. The line beside the button shows
where the rule will go: `Saves to` followed by the active rules file's name, or
`Saves to a new rules file` if none is active. In that case the app creates `rules.toml` in its config folder (see
[The active rules file](#the-active-rules-file)) and makes it active.

In Simple mode, the regular expression the tester generated is saved, not the
wildcard pattern. The pattern is written in single quotes, as recommended
above, unless it contains a single quote or a control character.

A saved rule is checked exactly like a rule in a loaded file. Nothing is
written if the pattern is invalid, if the id is already used in the file, or if
the file itself no longer loads. The error appears below the button, in the
same form as the errors in [Errors](#errors). A saved rule whose id matches a
builtin replaces that builtin, as in
[Replacing a builtin rule](#replacing-a-builtin-rule).

Saving adds the rule at the end of the file and leaves everything else as it
was, including comments, blank lines, and the order of existing rules. The
app reads the file from disk just before writing, so edits made in another
editor since it was loaded are kept. The file is replaced in one step, so a
crash during a save leaves either the old file or the new one, and an
existing file keeps its permissions.

After saving, the app loads the file again and rescans the open document.
This resets the review, as loading a rules file does, so any accept or reject
decisions are lost. If a review is in progress, the app asks for
confirmation before saving. If you cancel, nothing is saved and the id and
name you typed stay in place.

If the remembered rules file failed to load at startup, saving is refused
until a rules file loads successfully with **File → Load Rules**.
