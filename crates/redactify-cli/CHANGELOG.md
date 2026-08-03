## 0.6.1 (2026-08-03)

### Fixes

- Rules panel, menu shortcut fixes, and three rule false positives eliminated.

## 0.6.0 (2026-07-30)

### Breaking Changes

- the CLI now requires an explicit subcommand.
`redactify file.txt -o out.txt` is now `redactify scan file.txt -o
out.txt`. Introduced now rather than later since batch processing
(also on the v0.6 list) will want the same subcommand structure —
better to do this restructuring once.

### Features

- add redactify verify (#85)
- add redactify batch (#87)
- add JWT, Bitcoin address, Discord webhook, Mailchimp key (#89)

### Fixes

- fix Canadian SIN checksum bug and a wrong test assertion (#91)
