# ADR 003: Logging — desktop app only, typed events, no document data

Status: proposed · Date: 2026-10-10

## Context

The desktop app keeps no log. An error is shown once in the banner and is
gone when the banner clears, so a user filing a bug report has nothing to
attach and a failure that happened earlier cannot be investigated.

For most applications a log file is routine. For Redactify it is a second
place where sensitive data can be written to disk (CWE-532). A log line that
records a finding's matched text, a line of the document, or a file name puts
in a plain, unredacted file exactly what the user was trying to remove. The
sensitive data is not only in documents: custom rules can encode
organization secrets such as codenames or internal domains, and file paths
usually contain the user's name. Values that reach the log from user input
can also forge log lines if they contain line breaks (CWE-117).

The log must be designed so that none of this can happen by accident.

## Decisions

**Scope: the desktop app only.** The CLI keeps writing errors and summaries
to stderr. The core does not log; it returns errors and the front ends decide
what to do with them, consistent with the core having no I/O opinions.

**Mechanism: the official `tauri-plugin-log`, configured explicitly.** The
plugin's defaults are not relied on. Its targets are cleared and only the
`LogDir` target is added; development builds may also add `Stdout`. The
`Webview` target is never enabled. Logs go to the platform's log directory
for the app:

| OS | Location |
| --- | --- |
| Windows | `%LOCALAPPDATA%\com.garykellygrimm.redactify\logs` |
| macOS | `~/Library/Logs/com.garykellygrimm.redactify` |
| Linux | `$XDG_DATA_HOME/com.garykellygrimm.redactify/logs`, or `~/.local/share/…` when unset |

Files rotate at 1 MB with `RotationStrategy::KeepSome(2)`: the active file
plus two archives, about 3 MB in all. The plugin's default deletes the whole
file at its size limit, losing the history right before a failure.
`KeepSome(0)` is never used; the plugin does not guard against it. The bound
is approximate: two rotations within one second leave a `.bak` file that the
plugin's cleanup never removes.

On macOS and Linux the log directory is set to mode `0700` before the plugin
starts, so other local users cannot read it. The mode is set explicitly after
creating the directory, because a mode given at creation does not change a
directory left by an earlier build; a test checks the result. On Windows the
directory inherits the user profile's permissions, which allow the user,
SYSTEM, and Administrators.

**Level: `info` and above in release builds.** `debug` is available in
development builds only.

**Only Redactify's own code logs.** The plugin captures every crate that uses
the `log` facade, including Tauri and the webview libraries. A filter admits
only records whose target is exactly `app_lib` or starts with `app_lib::`.
Log macros never set an explicit `target:`, which would bypass the filter.
The frontend is not given the `log:default` permission.

**One entry point with typed events.** All logging goes through a single
function that takes a `LogEvent` enum. Its variants carry only allowlisted
values: counts, sizes, durations, rule ids, and error categories. Nothing
else in the app calls `log::info!` or its siblings directly. The compiler
then enforces most of the allowlist: a path or a line of text has no variant
to go in. CI makes a direct call a build failure: a `clippy.toml` in
`app/src-tauri/` configures three bans, and CI's `-D warnings` turns any
violation into an error.

- `disallowed-macros` bans `log::log`. Every level macro, from `trace!` to
  `error!`, expands through it, so banning it covers them all. The logging
  module allows the lint for itself.
- `disallowed-methods` bans `log::logger`, which reaches the logger through a
  function call that the macro lint cannot see.
- `disallowed-macros` also bans `std::println`, `std::eprintln`, and
  `std::dbg`, because a desktop app's stdout and stderr can still reach a
  system log.

The file sits in the app crate rather than the repository root so these bans
do not apply to the CLI, which writes to stderr by design.

**Errors are logged by category, not by message.** The messages commands
return to the user name files on purpose, such as `Could not read '<path>'`.
Commands return a typed error with a separate log-safe view: the error
variant and, for I/O, the `io::ErrorKind`. The user-facing message is never
logged.

**What may be logged:**

- The app version, OS, and architecture at startup.
- Which command ran and whether it succeeded or failed, by category.
- Counts and sizes: findings per rule, rules loaded, and file size in bytes.
- The file's type, from a fixed list such as `txt`, `log`, `csv`, and `json`,
  or `other`. The raw extension is not logged: in `report.John Smith`, the
  extension is `John Smith`.
- Elapsed time.
- Rule ids, escaped with `{:?}` so a line break in an id cannot forge a log
  line.
- For a rules file that fails to load: the error category, the rule id, and
  the line and column.

**What must never be logged:**

- Any document text: lines, segments, matched text, or surrounding context.
- Redacted output.
- Any path or file name: documents, output, manifests, or rules files.
- Rule patterns or any other rules-file content, including the TOML and regex
  error messages, which quote it.
- Anything typed into the pattern tester.
- User-facing error messages.
- `Debug` output of any value that holds the above.

**Panics: location only.** A panic hook logs the source file and line, not
the message. In release builds the hook replaces the default hook rather than
calling it, so the message is not printed anywhere. A panic message can
contain data: string slicing still reports the character at a bad index, and
`panic!`, `expect`, and a failed `unwrap` can include any value they were
given.

**Logs stay on the machine.** OWASP's Top 10 lists local-only logs as a
weakness, but that guidance targets server monitoring. For a user-owned tool
built for air-gapped use, with no telemetry, local-only is the deliberate
privacy choice.

**An automated test proves it.** The pull request that adds logging includes
a test that captures every log record while it runs the app's commands with
a canary string in the file names, the document text, and rule patterns. The
test fails if the canary appears in any record. Leaks are most likely in
error handling, so it covers failures as well as the normal path:

- opening, scanning, and saving a document
- opening a path that does not exist
- saving to an output path that cannot be written
- loading a rules file from a path that does not exist
- loading a rules file with an invalid pattern
- loading a rules file with a TOML syntax error
- previewing a pattern in the pattern tester
- a panic whose message contains the canary, which tests the panic hook

This turns the never-logged list into a CI check.

## Consequences

- Bug reports can include a log, and the user can read the whole file before
  sharing it. The bug report template should say where to find it.
- A log entry such as "open failed: NotFound" cannot say which file, and a
  rules error in the log gives a line number but not the line. The user sees
  the full message in the app; the log cannot leak what it never records.
- Rule ids are logged, and an id can itself name something sensitive. This
  matches the manifest, which already records the ids of the rules applied.
  Teams that treat their rule ids as sensitive should name them neutrally,
  and the custom rules guide should say so where rule authors will see it.
- Third-party crates are silent in the log, so a failure inside Tauri itself
  leaves no trace there. When one is needed, it is enabled for that crate in
  a development build, never in a release.
- Outside the log, the app already stores the full paths of the last five
  documents in `recent_files.json` in its data directory, for **File → Open
  Recent**. That is accepted: the list is the feature, and **Clear Recent
  Files** empties it.
- The CLI's `batch` command prints each input path to stderr, and CI systems
  keep job logs. A CLI option that omits paths is a follow-up.
- The typed entry point, the target filter, and the canary test enforce the
  never-logged list together; review covers what they cannot.
