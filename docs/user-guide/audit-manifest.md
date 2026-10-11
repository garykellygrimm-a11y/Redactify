# The audit manifest

Every export from the desktop app writes an audit manifest, and so does the
CLI: `scan` when given `--manifest`, and `batch` for each file when given
`--output-dir`. A manifest is a JSON record of what was detected, what was
redacted, and which exact files were involved. It lets someone confirm a
redaction without trusting the person who did it.

## What it records

A shortened example:

```json
{
  "tool": "redactify 0.7.1",
  "created_utc": "2026-10-11T18:48:11Z",
  "source_sha256": "c62fbcef8d3e7b836dc71575bd5d9b6f655bb579ec670e3f38a4a71b356a8245",
  "output_sha256": "e3538b96c46ce68a5ba094a4ec08ac54e006ace0bedd849102fd24da9e2246b7",
  "rules_applied": ["email", "ipv4", "ssn"],
  "finding_count": 2,
  "applied_count": 2,
  "findings": [
    { "rule_id": "email", "start": 31, "end": 51, "length": 20, "disposition": "accepted" },
    { "rule_id": "ipv4", "start": 57, "end": 70, "length": 13, "disposition": "accepted" }
  ]
}
```

A real manifest lists every active rule in `rules_applied` and every finding
in `findings`.

- **The tool and time** that produced it.
- **A SHA-256 hash of the complete original and of the complete output.**
  These tie the manifest to exactly two files.
- **The id of every active rule**, whether or not it matched anything.
- **Every finding:** which rule found it, where it is, and its disposition.
  `accepted` means it was redacted. `rejected` means a reviewer saw it and
  chose to keep it. `start` and `end` are byte offsets in the original file,
  which differ from character positions in text with non-ASCII characters.

Every field is defined in [Manifest format](../reference/manifest-format.md).

## What it never records

The manifest contains no matched text and no hash of any individual finding.
Sensitive values like Social Security numbers and phone numbers have so few
possible values that a hash of one can be reversed by trying them all, so a
per-finding hash would leak the very data that was redacted. Someone holding
only the manifest learns which rules fired and where, but nothing about the
values. See [ADR 001](../adr/001-manifest-content.md) for the full reasoning.

## Desktop app and CLI manifests

The desktop app records each finding as `accepted` or `rejected`, according to
the reviewer's decision.

The CLI has no review step and redacts every finding, so its manifests record
every finding as `accepted`. That is accurate rather than a default: no human
decided otherwise.

## Verifying a redaction

### With Redactify

Given the original, the output, and the manifest, `verify` checks all three
against each other:

```console
$ redactify-cli verify original.log --output clean.log --manifest clean.log.manifest.json
✓ source file matches manifest
✓ output file matches manifest
✓ every finding is accounted for (redaction reconstructs exactly)

verified: this manifest's account of what happened checks out.
```

The third check rebuilds the output from the original and the manifest's
accepted findings, then compares it byte for byte with the output file. It
catches a manifest edited after the fact, such as a finding changed from
`accepted` to `rejected`, which the hashes alone would miss. See
[Command line](cli.md#verify).

### With standard tools

The two hashes can be checked without Redactify. The result must match
`source_sha256` and `output_sha256`:

```console
$ sha256sum original.log clean.log
```

On Windows:

```powershell
Get-FileHash original.log, clean.log -Algorithm SHA256
```

`Get-FileHash` prints the hash in uppercase; the manifest uses lowercase.
Compare them without regard to case.

## What verification proves, and what it doesn't

A successful verification proves that the original, the output, and the
manifest are consistent: this output is exactly the original with the
manifest's accepted findings redacted.

It does not prove who produced the manifest, or that Redactify did. The
manifest is not signed, so anyone holding the original and the output could
write a matching one. Keep the manifest with the files it describes, and treat
its origin the way you would treat the origin of the files themselves.

It also does not prove that the rules found everything sensitive. It records
what the active rules found and what a reviewer decided, nothing more.
