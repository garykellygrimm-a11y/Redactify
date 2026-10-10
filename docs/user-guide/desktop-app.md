# Desktop app

The desktop app finds sensitive data in a text file, lets you review every
finding, and exports a redacted copy with an audit manifest. Nothing is
redacted until you have decided every finding.

## Opening a file

Open a text file in any of these ways:

- drag it onto the window
- click **Browse**
- **File → Open** (`Ctrl+O`)
- **File → Open Recent**, which lists the last five files you opened

The file is scanned as soon as it opens. Every finding is highlighted in the
document, colored by the rule that found it.

If a review is in progress, opening another file, closing the document, or
loading new rules asks for confirmation first, because it discards your
decisions.

## The window

- **The sidebar** has two tabs. **Findings** lists every finding, grouped by
  rule. **Rules** lists every active rule and, in releases after 0.6.2, holds
  the pattern tester; see [Custom rules](custom-rules.md).
- **The document** shows the file with each finding highlighted. Accepted
  findings appear as `[REDACTED:<rule_id>]`, so the document is a live preview
  of the output.
- **The status strip** at the bottom counts accepted, rejected, and pending
  findings, and holds the **Export** button.

## Reviewing findings

Every finding starts as pending. Use the keyboard to work through them:

1. `↑` and `↓` move between findings, and the document scrolls to follow.
2. `a` accepts the focused finding, which means it will be redacted. `r`
   rejects it, which means it stays in the output. Either one moves to the
   next finding.
3. `Shift+A` and `Shift+R` accept or reject every pending finding of the
   focused finding's rule at once. They never change a finding you already
   decided.
4. `Ctrl+Z` undoes the last decision. Press it again to keep going back. A
   rule-wide decision undoes in one step.

Rejecting a finding is a recorded decision, not an omission: the manifest
records that a reviewer saw it and chose to leave it.

## Checking the output

**Before/after preview** (`Ctrl+D`, or **View → Before / After Preview**)
switches between the original document and the output as it would export
right now.

**Search** (`Ctrl+F`) finds text in the document. `Esc` closes it.

## Exporting

You can export once no findings are pending.

- **Export** (`Ctrl+E`, or **File → Export**) always asks where to save.
- **Save** (`Ctrl+S`) reuses the last export destination from this session,
  and asks the first time.

The **Export** button in the status strip is disabled while any finding is
pending. It is also disabled for a file with no findings at all; use `Ctrl+E`
or **File → Export** to export such a file with its manifest.

Each export writes two files:

- the redacted output, at the path you chose
- the audit manifest, at the same path with `.manifest.json` added, for
  example `report.clean.log.manifest.json`

See [The audit manifest](audit-manifest.md) for what the manifest records and
how to verify it.

## Appearance

- **Theme:** `Ctrl+T`, or **View → Toggle Theme**, switches between light and
  dark.
- **Text size:** the `A−` and `A+` buttons in the top bar make the document
  text smaller or larger. The label between them shows the current size.

Both settings, and the window's size and position, are remembered between
launches.

## Keyboard shortcuts

On macOS, use `Cmd` wherever this table shows `Ctrl`. Press `?` in the app to
see this list.

| Keys | Action |
| --- | --- |
| `↑` / `↓` | Move between findings |
| `a` / `r` | Accept / reject the focused finding |
| `Shift+A` / `Shift+R` | Accept / reject every pending finding of the rule |
| `Ctrl+Z` | Undo the last decision |
| `Ctrl+F` | Search the document |
| `Ctrl+D` | Toggle before/after preview |
| `Ctrl+S` | Save, reusing the last export destination |
| `Ctrl+E` | Export, always asking where to save |
| `Ctrl+O` | Open a file |
| `Ctrl+L` | Load a custom rules file |
| `Ctrl+W` | Close the current document |
| `Ctrl+T` | Toggle light/dark theme |
| `?` | Open or close the shortcuts panel |
| `Esc` | Close search or the shortcuts panel |
