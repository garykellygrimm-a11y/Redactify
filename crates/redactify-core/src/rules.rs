use std::collections::HashSet;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RedactifyError;

/// Maximum compiled size for a user-supplied pattern, in bytes.
///
/// Rust's regex engine is immune to catastrophic backtracking (matching is
/// guaranteed linear-time), so the only resource a hostile pattern can
/// attack is memory at compile time. This cap closes that door. Builtins
/// are exempt: we wrote them, and they compile in tests.
const USER_PATTERN_SIZE_LIMIT: usize = 1 << 20; // 1 MiB

/// A detection rule: a named, compiled pattern.
#[derive(Debug)]
pub struct Rule {
    /// Stable machine id used in findings and redaction tokens, e.g. "ipv4".
    pub id: String,
    /// Human-readable name for UI/reports.
    pub name: String,
    /// Compiled regex. Note: Rust's `regex` crate has NO lookahead/lookbehind,
    /// so all patterns lean on `\b` word boundaries instead.
    pub pattern: Regex,
    /// Optional structural check beyond the pattern — a real checksum.
    /// Builtins only: a function pointer cannot come from a TOML file.
    pub validator: Option<fn(&str) -> bool>,
    /// If set, only this capture group's span becomes the finding — not
    /// the whole match. Lets a rule require surrounding context for
    /// precision (e.g. requiring a full `scheme://user:PASSWORD@host`
    /// shape) while only flagging/redacting the actual secret portion
    /// within it. `None` (the default every existing rule uses) means
    /// "the whole match is the finding," unchanged from before this
    /// field existed. Builtins only, same reasoning as `validator`: a
    /// TOML rules file has no way to express "use group N."
    pub finding_group: Option<usize>,
}

/// A serializable view of a [`Rule`].
///
/// [`Rule`] itself can't cross a serialization boundary: it holds a
/// compiled `Regex` and, for some rules, a function pointer. This carries
/// the parts a UI actually needs to display or edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleInfo {
    pub id: String,
    pub name: String,
    pub pattern: String,
    /// True when the rule runs a real check beyond the pattern match —
    /// Luhn, IBAN mod-97, Base58Check, the JWT header decode. Worth
    /// surfacing: these rules are far less prone to false positives than
    /// shape-only ones, and that's not visible from the pattern alone.
    pub validated: bool,
    /// Set when only a capture group becomes the finding rather than the
    /// whole match (currently just db_connection_string, which matches a
    /// whole connection string for precision but flags only the password).
    pub finding_group: Option<usize>,
}

impl Rule {
    /// Build the serializable view of this rule.
    pub fn info(&self) -> RuleInfo {
        RuleInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            // as_str() gives back the exact pattern the Regex was compiled
            // from, so this round-trips to the same rule.
            pattern: self.pattern.as_str().to_string(),
            validated: self.validator.is_some(),
            finding_group: self.finding_group,
        }
    }

    /// Convenience constructor for builtins. `.expect()` is deliberate:
    /// these are OUR hardcoded patterns — if one doesn't compile, we want
    /// tests to explode immediately, not limp along silently missing a rule.
    fn new(id: &str, name: &str, pattern: &str) -> Rule {
        Rule {
            id: id.to_string(),
            name: name.to_string(),
            pattern: Regex::new(pattern).expect("builtin pattern must compile"),
            validator: None,
            finding_group: None,
        }
    }

    /// Like `new`, but with a post-match validator attached. Use sparingly
    /// — only when the regex alone would be too liberal to be useful (a
    /// bare 13-19 digit run matches constantly) and a real checksum
    /// exists to narrow it down.
    fn with_validator(id: &str, name: &str, pattern: &str, validator: fn(&str) -> bool) -> Rule {
        Rule {
            id: id.to_string(),
            name: name.to_string(),
            pattern: Regex::new(pattern).expect("builtin pattern must compile"),
            validator: Some(validator),
            finding_group: None,
        }
    }

    /// Both mechanisms at once: capture group `group` becomes the finding,
    /// and `validator` gets the final say. ipv6 needs both — a guard
    /// character consumed to stand in for the lookbehind Rust's regex
    /// engine lacks, plus a structural check the pattern can't express.
    fn with_group_and_validator(
        id: &str,
        name: &str,
        pattern: &str,
        group: usize,
        validator: fn(&str) -> bool,
    ) -> Rule {
        Rule {
            id: id.to_string(),
            name: name.to_string(),
            pattern: Regex::new(pattern).expect("builtin pattern must compile"),
            validator: Some(validator),
            finding_group: Some(group),
        }
    }

    /// Like `new`, but only capture group `group` becomes the finding,
    /// not the whole match. Use when precision genuinely requires
    /// context the finding itself shouldn't include — e.g. matching a
    /// full `scheme://user:PASSWORD@host` connection string for
    /// specificity, while only flagging/redacting the password.
    fn with_group(id: &str, name: &str, pattern: &str, group: usize) -> Rule {
        Rule {
            id: id.to_string(),
            name: name.to_string(),
            pattern: Regex::new(pattern).expect("builtin pattern must compile"),
            validator: None,
            finding_group: Some(group),
        }
    }
}

/// The default rule set shipped with Redactify.
pub fn builtin_rules() -> Vec<Rule> {
    vec![
        // local-part@domain.tld — deliberately pragmatic, not RFC 5322.
        Rule::new(
            "email",
            "Email Address",
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b",
        ),
        // Four octets, each 0-255, dot-separated.
        Rule::new(
            "ipv4",
            "IPv4 Address",
            r"\b(?:(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)\b",
        ),
        // DECISION (documented): match liberally on shape (###-##-####),
        // don't validate SSA area-number rules. False positives are
        // acceptable because a human reviews findings.
        Rule::new("ssn", "US Social Security Number", r"\b\d{3}-\d{2}-\d{4}\b"),
        // Separators between groups are required, not optional — otherwise
        // this matches any bare 10-digit run, i.e. every timestamp in a log.
        // Cost: an unformatted 5551234567 no longer matches.
        Rule::new(
            "us_phone",
            "US Phone Number",
            r"(?:\+?1[-. ])?(?:\(\d{3}\)[-. ]?|\b\d{3}[-. ])\d{3}[-. ]\d{4}\b",
        ),
        // AWS access key IDs: AKIA (long-term) or ASIA (temporary) + 16
        // uppercase alphanumerics.
        Rule::new(
            "aws_access_key",
            "AWS Access Key ID",
            r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b",
        ),
        // Does not cover Google's newer "AQ." format, whose length and
        // charset were not documented when this was written.
        Rule::new("gcp_api_key", "GCP API Key", r"\bAIza[0-9A-Za-z\-_]{35}\b"),
        // Google OAuth 2.0 client ID: numeric project prefix + random
        // string + the long-standing, stable .apps.googleusercontent.com
        // suffix.
        Rule::new(
            "gcp_oauth_client_id",
            "GCP OAuth Client ID",
            r"\b\d{6,}-[0-9a-z]{20,}\.apps\.googleusercontent\.com\b",
        ),
        // Oracle Cloud Identifier: ocid1.<resource-type>.<realm>.
        // [region][.future-use].<unique-id>. Region may be empty (two
        // consecutive dots, as in tenancy OCIDs) — the pattern allows
        // that rather than requiring a non-empty region segment.
        Rule::new(
            "oracle_ocid",
            "Oracle Cloud Identifier",
            r"\bocid1\.[a-z0-9]+\.[a-z0-9]+\.[a-z0-9-]*\.[a-z0-9]+\b",
        ),
        // Requires both sv= and sig=. Relies on their conventional ordering,
        // since Rust's regex has no lookaround to assert co-occurrence.
        Rule::new(
            "azure_sas_token",
            "Azure SAS Token",
            r"\bsv=\d{4}-\d{2}-\d{2}[^\s]*?[&?]sig=[A-Za-z0-9%/+=]{20,}",
        ),
        // Generic PEM private-key block header. Not brand-specific —
        // catches leaked keys from AWS, GCP, Oracle, and plain SSH alike
        // with one rule, since they all share this PEM preamble. Higher
        // signal-to-noise than most single-provider secret patterns:
        // AWS Secret Access Keys and Azure Storage Account Keys are
        // deliberately NOT included as separate rules — both are opaque
        // base64 blobs with no fixed prefix, and matching "any 40+ char
        // base64-looking string" would mean constant false positives.
        Rule::new(
            "private_key_block",
            "PEM Private Key Block",
            r"-----BEGIN (RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----",
        ),
        // Stripe secret/publishable/restricted keys: fixed sk_/pk_/rk_
        // prefix + live/test mode + random suffix. High precision, and
        // payment-processor keys are about as consequential a thing to
        // catch as this tool deals with.
        Rule::new(
            "stripe_api_key",
            "Stripe API Key",
            r"\b(?:sk|pk|rk)_(?:live|test)_[A-Za-z0-9]{10,}\b",
        ),
        // DigitalOcean tokens: dop_v1_ (personal access token), doo_v1_
        // (OAuth flow), or dor_v1_ (OAuth refresh token), each + 64 hex.
        Rule::new(
            "digitalocean_token",
            "DigitalOcean API Token",
            r"\bdo[opr]_v1_[0-9a-f]{64}\b",
        ),
        // SendGrid API key: SG. + two dot-separated segments, ~69 chars
        // total. Ranges are generous since published lengths vary
        // slightly (68-70) across sources.
        Rule::new(
            "sendgrid_api_key",
            "SendGrid API Key",
            r"\bSG\.[A-Za-z0-9_-]{20,24}\.[A-Za-z0-9_-]{40,50}\b",
        ),
        // GitHub tokens: classic (ghp_/gho_/ghu_/ghs_/ghr_ + 36 alnum)
        // and fine-grained (github_pat_ + 80+ alnum/underscore).
        Rule::new(
            "github_token",
            "GitHub Token",
            r"\b(?:gh[oprsu]_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{80,})\b",
        ),
        // Slack tokens: xox[baprs]- + digit/alnum segments. The suffix
        // length floor is deliberate — a real-world secret-scanner bug
        // (GitLab) made the suffix optional and ended up matching the
        // bare literal "xoxb-" with nothing after it.
        Rule::new(
            "slack_token",
            "Slack Token",
            r"\bxox[baprs]-[0-9a-zA-Z-]{10,}\b",
        ),
        // HashiCorp Vault tokens (1.10+): hvs./hvb./hvr. + 24+ alnum,
        // per the format documented directly by Vault maintainers.
        Rule::new(
            "hashicorp_vault_token",
            "HashiCorp Vault Token",
            r"\bhv[sbr]\.[A-Za-z0-9]{24,}\b",
        ),
        // Two valid shapes: the full 8-group form, or one containing "::".
        // The leading guard stands in for lookbehind (which Rust's regex
        // lacks) so C++/Rust scope resolution stops matching; the validator
        // rejects 8-group hardware addresses.
        Rule::with_group_and_validator(
            "ipv6",
            "IPv6 Address",
            concat!(
                r"(?:^|[^0-9A-Za-z_:])(",
                r"(?:[0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{1,4}){7}",
                r"|[0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{1,4}){0,6}::(?:[0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{1,4}){0,6})?",
                r"|:(?::[0-9a-fA-F]{1,4}){1,7}))",
            ),
            1,
            ipv6_is_not_hardware_address,
        ),
        // Requires a documented sub-prefix rather than a bare "sk-", which
        // would also match every Anthropic key below. No fixed length —
        // OpenAI has changed it.
        Rule::new(
            "openai_api_key",
            "OpenAI API Key",
            r"\bsk-(?:proj|svcacct|admin)-[A-Za-z0-9_-]{20,}\b",
        ),
        // Anthropic API key: sk-ant- + api03 (standard) or oat01 (OAuth)
        // + ~95-char body. Requiring the generation tag (api03/oat01)
        // rather than a bare "sk-ant-" keeps this from being any looser
        // than it needs to be, mirroring the OpenAI rule's discipline.
        Rule::new(
            "anthropic_api_key",
            "Anthropic API Key",
            r"\bsk-ant-(?:api03|oat01)-[A-Za-z0-9_-]{60,}\b",
        ),
        Rule::new("npm_token", "npm Access Token", r"\bnpm_[A-Za-z0-9]{36}\b"),
        // Twilio Account SID / API Key SID: AC/SK + 32 hex. Lower
        // confidence than the other rules here — a 2-character prefix is
        // less distinctive than the 4+ character prefixes everything
        // else in this file uses, so it has a slightly higher chance of
        // colliding with unrelated hex-looking identifiers.
        Rule::new("twilio_sid", "Twilio SID", r"\b(?:AC|SK)[0-9a-fA-F]{32}\b"),
        // Credit/debit card number: a bare 13-19 digit run (with optional
        // single space/hyphen separators between digits) matches
        // constantly on its own — order numbers, invoice IDs, phone
        // sequences. Luhn is the actual signal here, not the regex; the
        // pattern is intentionally liberal about grouping/separators
        // since the checksum is what does the real work of ruling out
        // non-card numbers.
        Rule::with_validator(
            "credit_card",
            "Credit/Debit Card Number",
            concat!(
                r"\b(?:4\d{12}(?:\d{3})?|5[1-5]\d{14}|3[47]\d{13}|6(?:011|5\d{2})\d{12}",
                r"|2(?:2[2-9]|[3-6]\d|7[01])\d{12}|2720\d{12}",
                r"|4\d{3}[ -]\d{4}[ -]\d{4}[ -]\d{4}",
                r"|5[1-5]\d{2}[ -]\d{4}[ -]\d{4}[ -]\d{4}",
                r"|3[47]\d{2}[ -]\d{6}[ -]\d{5})\b",
            ),
            luhn_valid,
        ),
        // Shape alone is far too liberal; the validator decodes the header
        // and confirms it is JSON containing "alg".
        Rule::with_validator(
            "jwt",
            "JSON Web Token",
            r"\b[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b",
            jwt_header_is_valid,
        ),
        // Bitcoin address (P2PKH/P2SH, legacy Base58Check forms — not
        // the newer Bech32 "bc1..." SegWit format, which uses a
        // different checksum scheme entirely and would need its own
        // rule). Real checksum: the last 4 bytes of the decoded 25 bytes
        // must equal the first 4 bytes of double-SHA256 of the
        // preceding 21 bytes. Verified against real, publicly-known
        // addresses (the Bitcoin genesis block address, and a
        // well-known P2SH address) rather than constructed test data.
        Rule::with_validator(
            "bitcoin_address",
            "Bitcoin Address",
            r"\b[13][1-9A-HJ-NP-Za-km-z]{25,34}\b",
            bitcoin_address_is_valid,
        ),
        // Discord webhook URL: fixed domain + path structure with a
        // Snowflake id (17-19 digits) and an opaque token. High
        // precision given the fixed https://discord(app).com/api/.../
        // webhooks/ prefix — about as distinctive as a URL-shaped rule
        // gets.
        Rule::new(
            "discord_webhook",
            "Discord Webhook URL",
            r"https://(?:canary\.|ptb\.)?discord(?:app)?\.com/api(?:/v\d+)?/webhooks/\d{17,19}/[\w-]+",
        ),
        // Mailchimp API key: 32-char hex + a datacenter suffix
        // (-us1..-us21 etc). The suffix is mandatory for the key to
        // actually work, so requiring it here is not just liberal
        // matching — a bare 32-char hex string with no suffix isn't a
        // valid Mailchimp key at all.
        Rule::new(
            "mailchimp_api_key",
            "Mailchimp API Key",
            r"\b[0-9a-f]{32}-[a-z]{2}\d{1,2}\b",
        ),
        // IBAN: 2-letter country + 2 check digits + up to 30 alnum BBAN
        // characters, validated via ISO 7064 MOD 97-10 (move the first
        // 4 chars to the end, letters -> numbers A=10..Z=35, the result
        // interpreted as one big integer must be ≡ 1 mod 97). Verified
        // against a well-known example IBAN used throughout ISO/banking
        // documentation (GB82 WEST...), not constructed test data.
        Rule::with_validator(
            "iban",
            "IBAN",
            r"\b[A-Z]{2}\d{2}(?:[ ]?[A-Z0-9]){11,30}\b",
            iban_is_valid,
        ),
        // US bank routing number (ABA): 9 digits, weighted mod-10
        // checksum (3-7-1 repeating). Verified against a real, public
        // routing number — these identify banks, not individual
        // accounts, so there's no privacy concern using a real one, the
        // same way a company's public IP address isn't sensitive.
        Rule::with_validator(
            "us_routing_number",
            "US Bank Routing Number",
            r"\b\d{9}\b",
            us_routing_number_is_valid,
        ),
        // Canadian SIN: 9 digits, Luhn — the exact same checksum
        // credit_card uses, but through its own validator rather than
        // calling luhn_valid directly, since that function's length
        // bound (13-19, correct for card numbers) would reject every
        // 9-digit SIN outright before the checksum was ever checked.
        Rule::with_validator(
            "canadian_sin",
            "Canadian Social Insurance Number",
            r"\b\d{9}\b",
            canadian_sin_is_valid,
        ),
        // Matches the whole connection string for precision but flags only
        // the captured credential. Username is optional for Redis's
        // redis://:password@host form.
        Rule::with_group(
            "db_connection_string",
            "Database Connection String Credential",
            r"(?:postgres(?:ql)?|mysql|mongodb(?:\+srv)?|redis|amqp)://[^:@\s/]*(:[^@\s]+)@[^\s/]+",
            1,
        ),
    ]
}

/// Luhn checksum — the standard validity check for payment card numbers.
/// Strips non-digit separators first (real numbers are often written
/// grouped with spaces or hyphens), then requires a real card-shaped
/// length (13-19 digits) before checking the checksum itself.
fn luhn_valid(matched: &str) -> bool {
    let digits: Vec<u32> = matched
        .chars()
        .filter(|c| c.is_ascii_digit())
        .filter_map(|c| c.to_digit(10))
        .collect();
    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }
    luhn_checksum_valid(&digits)
}

fn luhn_checksum_valid(digits: &[u32]) -> bool {
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 1 {
                let doubled = d * 2;
                if doubled > 9 { doubled - 9 } else { doubled }
            } else {
                d
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

/// Canadian SIN: exactly 9 digits, same Luhn algorithm as credit cards —
/// just its own, correct length requirement instead of borrowing
/// `luhn_valid`'s card-shaped 13-19 bound.
fn canadian_sin_is_valid(matched: &str) -> bool {
    let digits: Vec<u32> = matched
        .chars()
        .filter(|c| c.is_ascii_digit())
        .filter_map(|c| c.to_digit(10))
        .collect();
    if digits.len() != 9 {
        return false;
    }
    luhn_checksum_valid(&digits)
}

/// Reject MAC/EUI-64 hardware addresses, which are colon-separated hex
/// just like IPv6 and so match the full-form branch exactly.
///
/// The tell is uniformity: a hardware address is always eight groups of
/// exactly two hex digits, whereas real full-form IPv6 has groups of
/// varying width (2001:0db8:85a3:...). Anything containing "::" is
/// compressed IPv6 and can't be a hardware address at all.
fn ipv6_is_not_hardware_address(matched: &str) -> bool {
    if matched.contains("::") {
        return true;
    }
    let groups: Vec<&str> = matched.split(':').collect();
    !(groups.len() == 8 && groups.iter().all(|g| g.len() == 2))
}

/// Decode the JWT's header segment and confirm it's valid JSON containing
/// an "alg" key — the one field RFC 7519 guarantees every JWT header has.
fn jwt_header_is_valid(matched: &str) -> bool {
    let Some((header_b64, _rest)) = matched.split_once('.') else {
        return false;
    };
    let Ok(decoded) = URL_SAFE_NO_PAD.decode(header_b64) else {
        return false;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&decoded) else {
        return false;
    };
    value.get("alg").is_some()
}

/// Base58Check validation: decode, split into the 21-byte payload and its
/// 4-byte checksum, and confirm the checksum equals the first 4 bytes of
/// double-SHA256 of the payload.
fn bitcoin_address_is_valid(matched: &str) -> bool {
    let Ok(raw) = bs58::decode(matched).into_vec() else {
        return false;
    };
    if raw.len() != 25 {
        return false;
    }
    let (payload, checksum) = raw.split_at(21);
    let first_hash = Sha256::digest(payload);
    let second_hash = Sha256::digest(first_hash);
    &second_hash[..4] == checksum
}

/// ISO 7064 MOD 97-10: strip spaces, move the first 4 characters
/// (country code + check digits) to the end, convert letters to numbers
/// (A=10..Z=35), and check that the resulting decimal number ≡ 1 (mod 97).
/// Implemented digit-by-digit (the standard way to do this check without
/// needing bignum support) rather than parsing the whole rearranged
/// string into one integer, since IBANs can be up to 34 characters —
/// comfortably larger than fits in a u64 once every letter has expanded
/// to two digits.
fn iban_is_valid(matched: &str) -> bool {
    let cleaned: String = matched.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.len() < 15 || cleaned.len() > 34 {
        return false;
    }
    let chars: Vec<char> = cleaned.chars().collect();
    if !chars[0].is_ascii_uppercase() || !chars[1].is_ascii_uppercase() {
        return false;
    }
    if !chars[2].is_ascii_digit() || !chars[3].is_ascii_digit() {
        return false;
    }

    let rearranged = chars[4..].iter().chain(chars[0..4].iter());

    let mut remainder: u32 = 0;
    for &ch in rearranged {
        let value: u32 = if ch.is_ascii_digit() {
            ch.to_digit(10).unwrap()
        } else if ch.is_ascii_uppercase() {
            (ch as u32) - ('A' as u32) + 10
        } else {
            return false;
        };
        // Feed the value's digit(s) into the running remainder one
        // decimal digit at a time -- (remainder * 10 + digit) % 97 at
        // each step is equivalent to computing the mod of the whole
        // number, without ever needing more than a u32.
        if value >= 10 {
            remainder = (remainder * 10 + value / 10) % 97;
            remainder = (remainder * 10 + value % 10) % 97;
        } else {
            remainder = (remainder * 10 + value) % 97;
        }
    }
    remainder == 1
}

/// US bank routing number (ABA) checksum: 9 digits, weighted 3-7-1
/// repeating across the three groups of three, summed and checked for
/// divisibility by 10.
fn us_routing_number_is_valid(matched: &str) -> bool {
    let digits: Vec<u32> = matched.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 9 {
        return false;
    }
    let checksum = 3 * (digits[0] + digits[3] + digits[6])
        + 7 * (digits[1] + digits[4] + digits[7])
        + (digits[2] + digits[5] + digits[8]);
    checksum.is_multiple_of(10)
}

// ---------------------------------------------------------------------------
// User-defined rules
// ---------------------------------------------------------------------------

/// On-disk schema of a rules file. Separate from `Rule` because a `Regex`
/// can't be deserialized directly — patterns arrive as strings and must
/// survive validation before becoming a `Rule`.
#[derive(Debug, Deserialize)]
struct RuleFile {
    #[serde(default)]
    rules: Vec<RuleSpec>,
}

#[derive(Debug, Deserialize)]
struct RuleSpec {
    id: String,
    name: String,
    pattern: String,
}

/// Compile one user pattern under the size cap.
fn compile_user_pattern(
    id: &str,
    pattern: &str,
    size_limit: usize,
) -> Result<Regex, RedactifyError> {
    RegexBuilder::new(pattern)
        .size_limit(size_limit)
        .build()
        .map_err(|source| RedactifyError::InvalidRule {
            id: id.to_string(),
            source,
        })
}

/// Parse and validate rules from TOML text. Fail-fast by design: ANY
/// invalid or duplicate rule aborts the whole load. A partially-applied
/// rule set would produce output the user wrongly believes is fully
/// redacted — the one failure mode this tool must never have (ADR 002).
fn parse_rules(text: &str, size_limit: usize) -> Result<Vec<Rule>, RedactifyError> {
    let file: RuleFile = toml::from_str(text)?;

    let mut seen: HashSet<String> = HashSet::new();
    let mut rules = Vec::with_capacity(file.rules.len());

    for spec in file.rules {
        if !seen.insert(spec.id.clone()) {
            return Err(RedactifyError::DuplicateRuleId { id: spec.id });
        }
        let pattern = compile_user_pattern(&spec.id, &spec.pattern, size_limit)?;
        rules.push(Rule {
            id: spec.id,
            name: spec.name,
            pattern,
            // A Rust function pointer can't be expressed in a TOML rules
            // file — user-defined rules are shape-only, same as before
            // this field existed.
            validator: None,
            // Same story: no way to express "use capture group N" in
            // TOML either, so user rules are always whole-match.
            finding_group: None,
        });
    }

    Ok(rules)
}

/// Throwaway rule for previewing a candidate pattern, under the same size
/// cap as rules loaded from TOML.
pub fn compile_preview_rule(pattern: &str) -> Result<Rule, RedactifyError> {
    Ok(Rule {
        id: "preview".to_string(),
        name: "Preview".to_string(),
        pattern: compile_user_pattern("preview", pattern, USER_PATTERN_SIZE_LIMIT)?,
        validator: None,
        finding_group: None,
    })
}

/// Load user-defined rules from a TOML file.
pub fn load_rules_file(path: &Path) -> Result<Vec<Rule>, RedactifyError> {
    let text = std::fs::read_to_string(path).map_err(|source| RedactifyError::RulesIo {
        path: path.display().to_string(),
        source,
    })?;
    parse_rules(&text, USER_PATTERN_SIZE_LIMIT)
}

/// Merge user rules over builtins. A user rule whose id matches a builtin
/// REPLACES that builtin (teams may tighten or swap our defaults); all
/// other user rules are appended.
pub fn merge_rules(builtins: Vec<Rule>, user: Vec<Rule>) -> Vec<Rule> {
    let mut merged: Vec<Rule> = builtins
        .into_iter()
        .filter(|b| !user.iter().any(|u| u.id == b.id))
        .collect();
    merged.extend(user);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_rules_file_parses() {
        let text = r#"
[[rules]]
id = "cui_marker"
name = "CUI Banner Marking"
pattern = '(?i)\bCUI//[A-Z]+\b'

[[rules]]
id = "badge"
name = "Badge Number"
pattern = '\bBDG-\d{6}\b'
"#;
        let rules = parse_rules(text, USER_PATTERN_SIZE_LIMIT).expect("should parse");
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].id, "cui_marker");
        assert!(rules[1].pattern.is_match("BDG-123456"));
    }

    #[test]
    fn invalid_pattern_fails_and_names_the_rule() {
        let text = r#"
[[rules]]
id = "broken"
name = "Unclosed Group"
pattern = '(unclosed'
"#;
        match parse_rules(text, USER_PATTERN_SIZE_LIMIT) {
            Err(RedactifyError::InvalidRule { id, .. }) => assert_eq!(id, "broken"),
            other => panic!("expected InvalidRule, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_id_fails() {
        let text = r#"
[[rules]]
id = "twin"
name = "First"
pattern = 'a+'

[[rules]]
id = "twin"
name = "Second"
pattern = 'b+'
"#;
        match parse_rules(text, USER_PATTERN_SIZE_LIMIT) {
            Err(RedactifyError::DuplicateRuleId { id }) => assert_eq!(id, "twin"),
            other => panic!("expected DuplicateRuleId, got {other:?}"),
        }
    }

    #[test]
    fn size_limit_is_enforced() {
        // Any real pattern blows a 1-byte budget; proves the cap is wired.
        match compile_user_pattern("huge", r"(?:abc|def)+xyz", 1) {
            Err(RedactifyError::InvalidRule { id, .. }) => assert_eq!(id, "huge"),
            other => panic!("expected InvalidRule from size limit, got {other:?}"),
        }
    }

    #[test]
    fn user_rule_overrides_builtin_with_same_id() {
        let user = parse_rules(
            r#"
[[rules]]
id = "email"
name = "Corp Email Only"
pattern = '\b[a-z.]+@corp\.example\b'
"#,
            USER_PATTERN_SIZE_LIMIT,
        )
        .expect("should parse");

        let merged = merge_rules(builtin_rules(), user);
        assert_eq!(merged.len(), 30, "override must replace, not add");
        let email = merged.iter().find(|r| r.id == "email").expect("email rule");
        assert_eq!(email.name, "Corp Email Only");
        assert!(!email.pattern.is_match("bob@gmail.com"));
    }

    #[test]
    fn new_user_rules_append_to_builtins() {
        let user = parse_rules(
            r#"
[[rules]]
id = "badge"
name = "Badge Number"
pattern = '\bBDG-\d{6}\b'
"#,
            USER_PATTERN_SIZE_LIMIT,
        )
        .expect("should parse");

        let merged = merge_rules(builtin_rules(), user);
        assert_eq!(merged.len(), 31);
    }
}
