use std::collections::HashSet;
use std::path::Path;

use regex::{Regex, RegexBuilder};
use serde::Deserialize;

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
    /// Optional post-match check: given the matched text, does it actually
    /// look like a real instance of what this rule targets, beyond just
    /// matching the shape? Exists for the small set of rules where a real
    /// checksum (e.g. Luhn for card numbers) can cheaply rule out most
    /// false positives that a shape-only regex can't distinguish — it
    /// only ever runs on strings the regex already matched, so the cost
    /// is proportional to candidate count, not document size. Builtins
    /// only: a Rust function pointer can't be expressed in a TOML rules
    /// file, so user-defined rules always get `None` here (see
    /// `parse_rules` below).
    pub validator: Option<fn(&str) -> bool>,
}

impl Rule {
    /// Convenience constructor for builtins. `.expect()` is deliberate:
    /// these are OUR hardcoded patterns — if one doesn't compile, we want
    /// tests to explode immediately, not limp along silently missing a rule.
    fn new(id: &str, name: &str, pattern: &str) -> Rule {
        Rule {
            id: id.to_string(),
            name: name.to_string(),
            pattern: Regex::new(pattern).expect("builtin pattern must compile"),
            validator: None,
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
        // US phone: parenthesized area code is an explicit alternative so
        // the match anchors at '(' — \b cannot sit between space and paren.
        Rule::new(
            "us_phone",
            "US Phone Number",
            r"(?:\+?1[-. ]?)?(?:\(\d{3}\)|\b\d{3})[-. ]?\d{3}[-. ]?\d{4}\b",
        ),
        // AWS access key IDs: AKIA (long-term) or ASIA (temporary) + 16
        // uppercase alphanumerics.
        Rule::new(
            "aws_access_key",
            "AWS Access Key ID",
            r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b",
        ),
        // Google Cloud API key: fixed "AIza" prefix + 35 alnum/-/_, 39
        // chars total. Verified against current Google documentation.
        // NOTE: Google is mid-transition (as of mid-2026) to a second,
        // "AQ."-prefixed key format for keys issued via AI Studio: old
        // AIza keys are being phased out through September 2026. That
        // new format is not deliberately excluded — the length/charset
        // wasn't confidently pinned down from public docs at the time
        // this rule was written, and guessing wrong on a security-tool
        // pattern seemed worse than a documented gap. Revisit once
        // Google publishes the format.
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
        // Azure Shared Access Signature token: requires BOTH a dated
        // storage-service-version (sv=) parameter AND a signature
        // (sig=) parameter co-occurring, non-greedily spanning whatever
        // other query parameters sit between them. Rust's regex crate
        // has no lookaround, so this leans on the sv=...sig= ordering
        // being effectively universal in real SAS tokens (sig is
        // computed from the other parameters, so it's conventionally
        // emitted last) rather than asserting it independently of match
        // position.
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
        // IPv6 address, full and compressed (::) forms. Three branches,
        // each covering a structurally distinct, genuinely-valid shape:
        //   1. full form — exactly 8 groups, no compression
        //   2. compressed form — leading group(s), a literal "::", then
        //      optional trailing group(s)
        //   3. starts with a bare "::"
        //
        // This replaced an earlier, more liberal 2-branch version after
        // testing turned up a real bug, not just a nuisance: a
        // HH:MM:SS-shaped timestamp like "14:23:05" has 2 colons and
        // all-digit (= valid hex) groups, so a pattern that just counted
        // colons matched it as a plausible short IPv6 address. That
        // shape was never actually valid IPv6 in the first place — an
        // address without "::" compression must have exactly 8 groups,
        // full stop, so a 2-3 group address with single colons and no
        // "::" isn't a permissible shorter form, it's just invalid.
        // Requiring EITHER the full 8-group form OR an actual literal
        // "::" fixes this correctly rather than papering over it with a
        // higher minimum-colon-count heuristic — a real timestamp can
        // never contain a literal double colon, so this isn't a
        // precision/recall trade-off, it's excluding a shape that was
        // always wrong. Verified clean against HH:MM:SS timestamps,
        // decimal ratios, and MAC addresses (6 colon-separated hex
        // groups — structurally the closest look-alike).
        //
        // No \b boundaries: real addresses routinely start or end with a
        // colon (::1, 2001:db8::), and \b cannot sit between two
        // non-word characters — the same limitation already documented
        // on the us_phone rule above.
        //
        // Known, accepted residual trade-off: a single hex-letter/digit
        // "identifier" immediately followed by :: (e.g. the "d::" in
        // "std::io") can still false-positive, since a genuine minimal
        // leading group ("d::1") is structurally identical to that. Not
        // fixable without lookahead or a validator hook on Rule (see
        // the open Luhn-validation discussion — same underlying gap).
        // Left liberal here, consistent with this project's existing
        // policy of favoring recall over precision on shape-based rules.
        Rule::new(
            "ipv6",
            "IPv6 Address",
            concat!(
                r"(?:[0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{1,4}){7}",
                r"|[0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{1,4}){0,6}::(?:[0-9a-fA-F]{1,4}(?::[0-9a-fA-F]{1,4}){0,6})?",
                r"|:(?::[0-9a-fA-F]{1,4}){1,7})",
            ),
        ),
        // OpenAI API key: sk- + a documented modern sub-prefix + random
        // suffix. Deliberately requires one of the three current
        // sub-prefixes rather than also accepting bare legacy "sk-" keys
        // (no sub-prefix at all) — OpenAI is actively phasing those out,
        // and a bare "sk-" branch would collide with Anthropic's
        // "sk-ant-..." keys below (both start with "sk-", and without
        // this restriction the two rules would both match the same
        // Anthropic key, leaving detect()'s overlap resolution to decide
        // the label somewhat arbitrarily). No fixed length: OpenAI's key
        // length has changed at least once recently (one key reported
        // going from 56 to ~165 total characters between generations),
        // so a length floor is used instead of an exact count.
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
            r"\b\d(?:[ -]?\d){12,18}\b",
            luhn_valid,
        ),
    ]
}

/// Luhn checksum — the standard validity check for payment card numbers.
/// Strips non-digit separators first (real numbers are often written
/// grouped with spaces or hyphens), then sums digits from the right,
/// doubling every second one and subtracting 9 from any result over 9.
/// A valid number's total is divisible by 10.
fn luhn_valid(matched: &str) -> bool {
    let digits: Vec<u32> = matched
        .chars()
        .filter(|c| c.is_ascii_digit())
        .filter_map(|c| c.to_digit(10))
        .collect();
    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }
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
    sum % 10 == 0
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
        });
    }

    Ok(rules)
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
        assert_eq!(merged.len(), 22, "override must replace, not add");
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
        assert_eq!(merged.len(), 23);
    }
}
