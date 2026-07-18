# ADR 001: Audit manifest stores no finding content and no per-finding hashes

Status: accepted · Date: 2026-07-16

## Context

The audit manifest must let users prove a redaction occurred without the
manifest itself becoming a disclosure risk. Three candidate designs for
per-finding records were considered.

## Decision

Each manifest finding records only `rule_id`, `start`, `end`, and
`length`. Integrity is anchored at the document level: the manifest
stores the SHA-256 of the complete source text and of the complete
redacted output. A holder of the original file can verify the entire
chain (input → rules → output); a holder of only the manifest learns
nothing about the redacted values.

## Alternatives considered

**Store the matched text.** Rejected outright: the audit record would
contain exactly the data the tool exists to remove.

**Store an unsalted hash of each matched value.** Rejected. The data
this tool redacts is low-entropy: SSNs, phone numbers, and emails have
small, enumerable value spaces, so unsalted digests are trivially
reversible by dictionary/rainbow-table attack. A manifest of such
hashes is obfuscation, not protection.

**Store salted per-finding hashes.** Deferred, not rejected. Salting
defeats precomputation and would enable a future "verify that value X
was redacted" feature — but doing it correctly requires per-entry salt
handling and a verification protocol (cf. verifiable redactable audit
log schemes), i.e. key-management scope this release does not need.
May return as an explicit opt-in if users request value-level
verification.

## Consequences

- The manifest is safe to share with parties who must not see the
  original data (auditors, vendors).
- Value-level correlation across files is not possible from manifests
  alone — accepted trade-off at this stage.
- A regression test asserts serialized manifests never contain matched
  content.
