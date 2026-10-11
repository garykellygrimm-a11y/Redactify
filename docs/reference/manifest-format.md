# Manifest format

An audit manifest is a UTF-8 JSON object. The desktop app writes it beside the
exported output as `<output>.manifest.json`. The CLI's `scan` writes it to the
path given with `--manifest`, and `batch` writes one beside each output file as
`<output>.manifest.json`. For what the manifest is for and how to verify one,
see [The audit manifest](../user-guide/audit-manifest.md).

## Example

```json
{
  "tool": "redactify 0.7.1",
  "created_utc": "2026-10-11T18:48:11Z",
  "source_sha256": "c62fbcef8d3e7b836dc71575bd5d9b6f655bb579ec670e3f38a4a71b356a8245",
  "output_sha256": "fd2b73fc1e8ed8e6c4296bcc9922cc2c219201af518fdc6ff023413b4a5f137a",
  "rules_applied": ["email", "ipv4", "ssn"],
  "finding_count": 2,
  "applied_count": 1,
  "findings": [
    { "rule_id": "email", "start": 31, "end": 51, "length": 20, "disposition": "accepted" },
    { "rule_id": "ipv4", "start": 57, "end": 70, "length": 13, "disposition": "rejected" }
  ]
}
```

## Top-level fields

| Field | Type | Meaning |
| --- | --- | --- |
| `tool` | string | The tool and version that wrote the manifest, as `redactify <version>`. The desktop app and the CLI write the same value. |
| `created_utc` | string | When the manifest was written, in RFC 3339 format in UTC with whole seconds, such as `2026-10-09T00:50:49Z`. |
| `source_sha256` | string | SHA-256 of the complete original file, as 64 lowercase hex characters. |
| `output_sha256` | string | SHA-256 of the complete redacted output, as 64 lowercase hex characters. |
| `rules_applied` | array of strings | The id of every rule that was active, whether or not it matched, in the order the rules ran. |
| `finding_count` | integer | The number of findings. Equal to the length of `findings`. |
| `applied_count` | integer | The number of findings redacted in the output. Equal to the number of findings whose `disposition` is `accepted`. |
| `findings` | array of objects | Every finding, ordered by `start`. See below. |

## Finding fields

| Field | Type | Meaning |
| --- | --- | --- |
| `rule_id` | string | The id of the rule that produced the finding. |
| `start` | integer | Byte offset in the original file where the finding begins. |
| `end` | integer | Byte offset where the finding ends, exclusive. |
| `length` | integer | `end - start`, in bytes. |
| `disposition` | string | `accepted` if the finding was redacted, `rejected` if a reviewer kept it. |

Offsets count bytes of the UTF-8 file, not characters. In text containing
non-ASCII characters, such as accented letters or emoji, a byte offset is
larger than the character position. Findings never overlap.

## What is not included

The manifest never contains matched text or any hash of an individual
finding. See [ADR 001](../adr/001-manifest-content.md).

`rules_applied` records each rule's id but not its pattern. A custom rule that
replaces a builtin appears under the builtin's id.

The format has no version field. `tool` identifies the version of Redactify
that wrote the manifest.

## How verification uses the manifest

`redactify-cli verify` makes three independent checks:

1. **Source hash:** SHA-256 of the given original equals `source_sha256`.
   A failure means the wrong original was supplied, or it changed since the
   manifest was written.
2. **Output hash:** SHA-256 of the given output equals `output_sha256`.
   A failure means the wrong output was supplied, or it was altered after
   export.
3. **Redaction:** applying the `accepted` findings to the original reproduces
   the given output byte for byte. This is the check that proves every finding
   is accounted for, including one whose disposition was changed after the
   fact.

Verification passes only if all three pass.

## Reading manifests in other tools

The manifest is plain JSON and can be read by any JSON parser. When Redactify
reads a manifest, missing fields are an error and unrecognized fields are
ignored.
