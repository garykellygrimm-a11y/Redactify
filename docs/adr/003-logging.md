# ADR 003: Logging — desktop app only, allowlisted content, no document data

Status: proposed · Date: 2026-10-10

## Context

The desktop app keeps no log. An error is shown once in the banner and is
gone when the banner clears, so a user filing a bug report has nothing to
attach and a failure that happened earlier cannot be investigated.

For most applications a log file is routine. For Redactify it is a second
place where sensitive data can be written to disk. A log line that records a
finding's matched text, a line of the document, or an input file name puts in
a plain, unredacted file exactly what the user was trying to remove. The log
must be designed so that cannot happen by accident.

## Decisions

**Scope: the desktop app only.** The CLI keeps writing errors and summaries
to stderr, which a terminal or CI job already captures. The core does not
log; it returns errors and the front ends decide what to do with them,
consistent with the core having no I/O opinions.

**Mechanism: the official `tauri-plugin-log`.** It writes to the platform's
log directory for the app:

| OS | Location |
| --- | --- |
| Windows | `%LOCALAPPDATA%\com.garykellygrimm.redactify\logs` |
| macOS | `~/Library/Logs/com.garykellygrimm.redactify` |
| Linux | `~/.local/share/com.garykellygrimm.redactify/logs` |

Files rotate at 1 MB, keeping the three most recent. The plugin's default
discards the whole file at its size limit, which would lose the history
right before a failure. Logs never leave the machine.

**Level: `info` and above in release builds.** `debug` is available in
development builds.

**Only Redactify's own code logs.** The plugin captures every crate that
uses the `log` facade, including Tauri and the webview libraries, whose
messages are outside our control. A filter admits only records whose target
starts with `app_lib`, the app crate. The frontend is not given the
`log:default` permission; errors worth keeping arise in Rust commands and
are logged there.

**What may be logged:**

- The app version, OS, and architecture at startup.
- Which command ran and whether it succeeded or failed.
- Counts and sizes: findings per rule, rules loaded, file size in bytes,
  elapsed time.
- Rule ids, and the rules file's path.
- Error messages from rules files. These are configuration, not document
  data, and the user has already seen them.

**What is never logged:**

- Any document text: lines, segments, matched text, or surrounding context.
- Redacted output.
- Input and output document paths or file names. A file name can itself be
  sensitive, such as a person's name. Log the extension and size instead.
- `Debug` output of any value that holds document text, such as the open
  document or a finding with its text.

**Panics: location only.** A panic hook logs the source file and line, not
the panic message. Current Rust no longer includes string contents in
string-slicing panics, but a message built with `panic!`, `expect`, or a
failed `unwrap` can include any value it was given.

## Consequences

- Bug reports can include a log, and the user can read the whole file before
  sharing it. The bug report template should say where to find it.
- A log entry such as "open failed" cannot say which file. That is accepted:
  the user knows which file they opened, and a log that cannot name it also
  cannot leak it.
- The never-logged list is enforced by review, not by the compiler. Every
  pull request that adds a log call is checked against it.
- Third-party crates are silent in the log, so a failure inside Tauri itself
  leaves no trace there. If one is ever needed, it is enabled for that crate
  in a development build, not in releases.
