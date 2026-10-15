# ADR 004: Rule persistence — one remembered rules file, decisions kept across rescans

Status: proposed · Date: 2026-10-14

## Context

The desktop app loses its custom rules when it closes: a rules file loaded
with **File → Load Rules** must be loaded again after every restart. The
pattern tester can preview a pattern but cannot save it, so a tested pattern
has to be copied into a rules file by hand. And loading rules rescans the open
document and discards the whole review, so adding one rule mid-review throws
away every decision already made.

The remaining v0.7 items, saving a tested pattern as a rule and keeping review
decisions when a rule is added, both depend on deciding where rules live and
what happens to a review when the rule set changes.

Manifest provenance, meaning whether the manifest should record rule patterns
and carry a format version, is out of scope here and will be a separate ADR.

## Decisions

**One active rules file, remembered.** At any time the app has at most one
active rules file. Its path is stored in `settings.json` in the app's config
directory and loaded again at startup. **File → Load Rules** makes the chosen
file active and remembers it.

**A default file when none is active.** If no rules file is active when a rule
is saved, the app creates `rules.toml` in its config directory and makes it
active. Tauri's `app_config_dir` resolves to each platform's standard location
for app configuration:
[`$XDG_CONFIG_HOME` on Linux](https://specifications.freedesktop.org/basedir-spec/latest/),
[Roaming AppData on Windows](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid),
and [Application Support on macOS](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html)
([Tauri `PathResolver`](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html)).

**Saving writes to the active file.** The active file is the single source of
truth, including when it is a team's shared rules file. The CLI can use the
same file with `--rules`, so rules authored in the app work unchanged in a
pipeline. A rule is saved in four steps:

1. Read the active file from disk, so edits made in another editor since it
   was loaded are kept.
2. Validate the new rule exactly as a loaded rule is validated (ADR 002): the
   pattern must compile under the size limit, and its id must not already
   exist in the file. A rule that fails is not written, and the error names
   it.
3. Append the rule as a new `[[rules]]` table with
   [`toml_edit`](https://docs.rs/toml_edit/latest/toml_edit/), which preserves
   comments, whitespace, and the order of existing items. ADR 002 chose TOML
   partly so that teams can annotate why a rule exists; rewriting the file
   with a plain serializer would delete those comments.
4. Write atomically: write the new contents to a temporary file in the same
   directory, flush it to disk with `sync_all`, then rename it over the
   original. A rename within one filesystem replaces the file
   [atomically on POSIX](https://pubs.opengroup.org/onlinepubs/9799919799/functions/rename.html),
   [`tempfile`'s `persist`](https://docs.rs/tempfile/latest/tempfile/struct.NamedTempFile.html)
   provides the same replace, and the temporary file must share the
   destination's filesystem because
   [`fs::rename` cannot cross filesystems](https://doc.rust-lang.org/std/fs/fn.rename.html).
   A crash during a save leaves either the old file or the new one, never a
   truncated file.

A replacement file must keep the original's permissions, or a save would
quietly change who can read a team's rules. A new temporary file gets
[private `0600` permissions on Unix](https://docs.rs/tempfile/latest/tempfile/struct.Builder.html)
and, on Windows, only the permissions it
[inherits from its folder](https://learn.microsoft.com/en-us/training/modules/configure-manage-file-access/8-examine-permissions-when-copying-moving-files),
and a rename within a volume
[keeps the temporary file's permissions](https://devblogs.microsoft.com/oldnewthing/20060824-16/?p=29973),
not the original's. So on Unix the save copies the original file's mode to
the temporary file before the rename, and on Windows it replaces the file with
[`ReplaceFileW`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew),
which preserves the original's access control lists and its encryption.

After a successful save, the app reloads the active file from disk and
rescans, so what is scanned is always what the file says.

**A remembered file that fails to load is reported, not skipped.** If the
active file is missing or invalid at startup, the app shows the error in the
banner, scans with builtin rules only, and labels the rule set as builtins
only. The path stays remembered, so fixing the file and loading it again
restores it. Scanning silently without the rules the user expects is the
failure ADR 002 exists to prevent, and it is the case that
[failing fast](https://www.martinfowler.com/ieeeSoftware/failFast.pdf),
[OWASP's fail-safe principle](https://devguide.owasp.org/en/02-foundations/03-security-principles/),
and [PEP 20](https://peps.python.org/pep-0020/) ("Errors should never pass
silently") all argue against.

**Rules files are not encrypted by the app; they are protected like the
documents they apply to.** Custom rules can be sensitive, as ADR 003 notes,
but the documents Redactify opens are more sensitive still, and they sit on
the same disk in plain form. Encrypting the rules file would protect less than
it appears to:

- An app that encrypts its own files must keep the key where it can read it.
  On Windows, data protected with DPAPI can typically be decrypted by
  [any process running as the same user](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata),
  and malware running as the user can reach such keys, as the
  [Chromium team's account of local data encryption](https://textslashplain.com/2020/09/28/local-data-encryption-in-chromium/)
  describes. Which layer of encryption is appropriate
  [depends on the threat model](https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html).
- The threat that remains, a lost or stolen device, is addressed for every
  file at once by full-disk encryption, the control
  [CIS Safeguard 3.6](https://controls-assessment-specification.readthedocs.io/en/latest/control-3/control-3.6.html)
  names for end-user devices with sensitive data (BitLocker, FileVault,
  dm-crypt), and one of the storage encryption types in
  [NIST SP 800-111](https://csrc.nist.gov/pubs/sp/800/111/final).
- An encrypted rules file could not be read by the CLI, reviewed in a pull
  request, or annotated with comments, which removes the reasons for keeping
  rules in TOML.

Instead, files the app creates are private to the user: on macOS and Linux,
`rules.toml` and `settings.json` are created with mode `0600` and the config
directory with `0700`. Existing rules files keep the permissions and any
encryption their owners gave them, as described above. The documentation
recommends full-disk encryption for machines that hold sensitive documents.

**The CLI does not change.** It uses only the rules file passed with
`--rules` and never reads the app's settings or default file. The same
command on the same input must produce the same output wherever it runs, the
property [reproducible builds](https://reproducible-builds.org/docs/definition/)
depend on, and tools such as
[ripgrep](https://raw.githubusercontent.com/BurntSushi/ripgrep/master/GUIDE.md)
likewise never load configuration from a location the user did not name.

**Review decisions are kept when rules change.** A finding's identity is its
rule id with its start and end offsets. Detection is deterministic and returns
findings sorted and non-overlapping, so the same text and rules always produce
the same findings. When the rule set changes and the document is rescanned:

- A new finding with the same identity as an old one takes the old one's
  decision, accepted or rejected.
- Findings that no longer exist are dropped, and new findings start pending.
- The carried-over decisions become the review's starting snapshot, and the
  event log starts empty from that snapshot. Existing events are never
  rewritten: event logs are
  [append-only](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing),
  and state is rebuilt from
  [a snapshot plus later events](https://martinfowler.com/eaaDev/EventSourcing.html).
- The app reports what happened, for example "12 decisions kept, 3 new
  findings to review", in place of the current prompt to discard the review.

Rust computes the identity match, because the open document and both finding
lists live in Rust-side state. The rescan result tells the frontend, for each
new finding, which old finding it matches, if any. The frontend applies the
old decisions to build the snapshot; it does no offset comparison itself.

## Alternatives considered

**Save to an app-owned file separate from the loaded file.** Rejected. Rules
would live in two places, the CLI could not use the app's rules without
merging files, and which file a rule came from would become a question users
have to answer.

**Rewrite the event log's finding indices after a rescan.** Rejected. It would
keep undo working across a rescan, but rewriting stored events is
[a last resort](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
that breaks the log's immutability.

**Serialize the whole rules file with the `toml` crate.** Rejected. It drops
every comment in the file.

**Let the CLI fall back to the app's rules file.** Rejected. A pipeline's
result would depend on whether the desktop app had been used on that machine.

**Encrypt rules files in the app.** Rejected, for the reasons under the
decision above: the key would sit on the same machine, full-disk encryption
already covers device theft for every file, and the CLI and pull-request
review need readable TOML.

**Forget a remembered file that fails to load.** Rejected. The user would not
learn that their rules stopped applying.

## Consequences

- Undo stops at a rules change. Decisions made before it are kept, but they
  can no longer be undone one by one.
- A new rule can replace an existing finding through overlap resolution: the
  earliest start wins, and on a tie the longest match wins. The replaced
  finding's decision does not carry over, because its span changed, and the
  new finding is pending. A decision is never applied to a span it was not
  made for.
- The app can edit a team's shared rules file. Comments and layout are kept,
  with two exceptions in `toml_edit`: the order of dotted keys and a missing
  trailing newline are not preserved. Rules files use neither.
- Saving reads the file again just before writing, but there is no file lock.
  An edit made in another editor in the instant between that read and the
  rename can be lost. This is accepted for a single-user desktop tool.
- Rules are only as protected at rest as the machine they are on. A machine
  without full-disk encryption leaves its rules files, and its documents,
  readable to anyone who takes the disk.
- Preserving permissions on save needs platform-specific code: a mode copy on
  Unix and `ReplaceFileW` on Windows.
- `settings.json` stores the active rules file's path. Under ADR 003, that
  path is never written to the log.
- The CLI's behavior and the rules file format are unchanged.
