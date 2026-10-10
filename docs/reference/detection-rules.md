# Detection rules

Redactify has 30 builtin rules. Each finding in the app, in redacted output
(`[REDACTED:<id>]`), and in the manifest is labeled with the rule's id. A
[custom rule](../user-guide/custom-rules.md) with the same id replaces the
builtin.

Rules favor catching too much over missing something, because a person
reviews every finding in the desktop app. The rules marked with an
**additional check** run a test beyond the pattern, which makes their false
positives much rarer.

## Personal data

| Id | Name | Matches | Additional check |
| --- | --- | --- | --- |
| `email` | Email Address | `local-part@domain.tld`, matched pragmatically rather than to the full RFC 5322 grammar | |
| `ssn` | US Social Security Number | `###-##-####` | |
| `us_phone` | US Phone Number | Ten-digit US numbers with separators, such as `816-555-0142` or `(816) 555-0142` | |
| `canadian_sin` | Canadian Social Insurance Number | Nine-digit SINs | Luhn checksum |

## Network

| Id | Name | Matches | Additional check |
| --- | --- | --- | --- |
| `ipv4` | IPv4 Address | Four dot-separated octets, each 0–255 | |
| `ipv6` | IPv6 Address | The full eight-group form, or a compressed form containing `::` | Rejects hardware addresses shaped like IPv6 |

## Financial

| Id | Name | Matches | Additional check |
| --- | --- | --- | --- |
| `credit_card` | Credit/Debit Card Number | Visa, Mastercard (51–55 and 2-series), American Express, and Discover numbers of 13 to 16 digits. Visa, 51–55 Mastercard, and American Express are also matched when grouped with spaces or hyphens. | Luhn checksum |
| `iban` | IBAN | Country code, check digits, and account number | ISO 7064 mod-97 |
| `us_routing_number` | US Bank Routing Number | Nine-digit ABA routing numbers | ABA weighted checksum |
| `bitcoin_address` | Bitcoin Address | Legacy P2PKH and P2SH addresses | Base58Check checksum |

## Cloud and API credentials

| Id | Name | Matches | Additional check |
| --- | --- | --- | --- |
| `aws_access_key` | AWS Access Key ID | `AKIA` or `ASIA` followed by 16 characters | |
| `gcp_api_key` | GCP API Key | Google API keys | |
| `gcp_oauth_client_id` | GCP OAuth Client ID | Client IDs ending in `.apps.googleusercontent.com` | |
| `oracle_ocid` | Oracle Cloud Identifier | `ocid1.` identifiers, including those with an empty region | |
| `azure_sas_token` | Azure SAS Token | Shared access signatures containing both `sv=` and `sig=` | |
| `private_key_block` | PEM Private Key Block | PEM private-key headers from any provider, including SSH | |
| `stripe_api_key` | Stripe API Key | `sk_`, `pk_`, and `rk_` keys in live or test mode | |
| `digitalocean_token` | DigitalOcean API Token | `dop_v1_`, `doo_v1_`, and `dor_v1_` tokens | |
| `sendgrid_api_key` | SendGrid API Key | `SG.` keys | |
| `github_token` | GitHub Token | Classic (`ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_`) and fine-grained (`github_pat_`) tokens | |
| `slack_token` | Slack Token | `xoxb-`, `xoxa-`, `xoxp-`, `xoxr-`, and `xoxs-` tokens | |
| `hashicorp_vault_token` | HashiCorp Vault Token | `hvs.`, `hvb.`, and `hvr.` tokens (Vault 1.10 and later) | |
| `openai_api_key` | OpenAI API Key | OpenAI keys with a documented sub-prefix | |
| `anthropic_api_key` | Anthropic API Key | `sk-ant-api03-` and `sk-ant-oat01-` keys | |
| `npm_token` | npm Access Token | `npm_` followed by 36 characters | |
| `twilio_sid` | Twilio SID | `AC` or `SK` followed by 32 hex characters | |
| `mailchimp_api_key` | Mailchimp API Key | 32 hex characters with a datacenter suffix such as `-us21` | |
| `discord_webhook` | Discord Webhook URL | `discord.com` and `discordapp.com` webhook URLs | |
| `jwt` | JSON Web Token | Three dot-separated base64url segments | The header must decode to JSON containing `alg` |
| `db_connection_string` | Database Connection String Credential | The password in `postgres`, `mysql`, `mongodb`, `redis`, and `amqp` connection strings. Only the password, with its leading `:`, is flagged, not the whole string. | |

## Limits

Some rules deliberately leave cases out to avoid constant false positives:

- `us_phone` requires separators. An unformatted run like `5551234567` is not
  matched, because it would match every timestamp in a log.
- `ssn` checks the shape only, not Social Security Administration numbering
  rules.
- `credit_card` covers the card networks listed above. Numbers from other
  networks are not matched, and neither are Discover or 2-series Mastercard
  numbers written in groups.
- `gcp_api_key` does not cover Google's newer key format beginning `AQ.`.
- `bitcoin_address` does not cover Bech32 (`bc1...`) addresses, which use a
  different checksum.
- AWS secret access keys and Azure storage account keys have no rule. Both are
  base64 strings with no fixed prefix, and a pattern for them would match
  almost any long token.
- `twilio_sid` uses a two-character prefix, which is less distinctive than the
  other credential rules, so it is somewhat more likely to match an unrelated
  hex identifier.

To cover a case the builtins leave out, add a
[custom rule](../user-guide/custom-rules.md).
