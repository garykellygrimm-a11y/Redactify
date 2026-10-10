# Command line

The CLI scans text files and writes redacted output without a review step. It
suits scripts, pipelines, and bulk processing. For human review of each
finding, use the [desktop app](desktop-app.md).

The executable is `redactify-cli` (`redactify-cli.exe` on Windows). It
identifies itself as `redactify` in `--version` output; the command you run is
still `redactify-cli`.

It has three subcommands, and one is always required:

| Subcommand | Does |
| --- | --- |
| `scan` | Scan one file and write redacted output |
| `verify` | Check an original, its redacted output, and their manifest against each other |
| `batch` | Scan many files and directories in parallel |

Run `redactify-cli <subcommand> --help` for the full option list.

## scan

```console
$ redactify-cli scan input.log -o clean.log --manifest audit.json
6 findings: 1 aws_access_key, 1 email, 2 ipv4, 1 ssn, 1 us_phone
```

| Option | Effect |
| --- | --- |
| `-o`, `--output <path>` | Write the redacted output to a file. Without it, output goes to standard output. |
| `--manifest <path>` | Write the audit manifest to this path. |
| `--rules <path>` | Load custom rules from a TOML file. A rule with a builtin's id replaces that builtin. |
| `--no-builtins` | Use only the rules from `--rules`. Requires `--rules`. |

The redacted text goes to standard output and the findings summary goes to
standard error, so redirecting output captures only the clean text:

```console
$ redactify-cli scan input.log > clean.log
```

The CLI has no review step, so every finding is redacted, and its manifest
records every finding as `accepted`.

## verify

`verify` checks that an original file, a redacted output, and the manifest
written with it still agree. It confirms that both files match the hashes in
the manifest, then rebuilds the output from the original and the manifest's
accepted findings and compares the result byte for byte.

```console
$ redactify-cli verify input.log --output clean.log --manifest audit.json
✓ source file matches manifest
✓ output file matches manifest
✓ every finding is accounted for (redaction reconstructs exactly)

verified: this manifest's account of what happened checks out.
```

If any check fails, the failing lines show `✗`, the last line reads
`verification FAILED: the manifest does not match the given files.`, and the
command exits with status 1. A script can therefore use `verify` as a gate.
Manifests from the desktop app and from the CLI can both be verified.

## batch

```console
$ redactify-cli batch logs/ reports/ --recursive --output-dir clean/ -j 4
logs/app.log: 3 findings
reports/q1.txt: 1 findings

2 file(s) processed: 2 succeeded, 0 failed
```

| Option | Effect |
| --- | --- |
| `<inputs>...` | One or more files and directories. |
| `-r`, `--recursive` | Include subdirectories. Without it, only files directly inside each directory are scanned. |
| `--output-dir <dir>` | Write redacted output, and a manifest beside each file, into this directory. Without it, `batch` only reports findings. |
| `--rules <path>` | Custom rules, applied to every file. |
| `--no-builtins` | Use only the rules from `--rules`. Requires `--rules`. |
| `-j`, `--jobs <n>` | Use at most this many threads. The default is one per CPU core. |

### Where output goes

With `--output-dir`, each directory argument gets its own folder named after
it, and its structure is mirrored inside, so files from different input
directories never collide. A file given directly, rather than through a
directory, is written at the top of the output directory under its own name.
Each manifest is named after its output file with `.manifest.json` added.

### When something goes wrong

- **Before anything is scanned,** the run stops with an error if an input is
  neither a file nor a directory, or if the rules file cannot be loaded.
- **During the run,** a file that cannot be read is reported and skipped, and
  the rest continue. So is any input whose output path would be the same as
  another's, such as two files both named `app.log` given directly from
  different folders: both are skipped rather than one overwriting the other.
- **At the end,** if any file was skipped, the command exits with status 1, so
  a script does not mistake a partial run for a clean one.

## Exit status

| Status | Meaning |
| --- | --- |
| `0` | Success, or a verification that passed |
| `1` | An error, a verification that failed, or a batch in which any file was skipped |
| `2` | Invalid command-line usage, such as a missing subcommand or `--no-builtins` without `--rules` |

## Custom rules

`scan` and `batch` accept `--rules`. See [Custom rules](custom-rules.md) for
the file format.
