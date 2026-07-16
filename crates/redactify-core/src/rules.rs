use regex::Regex;

/// A detection rule: a named, compiled pattern.
pub struct Rule {
    /// Stable machine id used in findings and redaction tokens, e.g. "ipv4".
    pub id: String,
    /// Human-readable name for UI/reports.
    pub name: String,
    /// Compiled regex. Note: Rust's `regex` crate has NO lookahead/lookbehind,
    /// so all patterns lean on `\b` word boundaries instead.
    pub pattern: Regex,
}

impl Rule {
    /// Convenience constructor. `.expect()` is deliberate: these are OUR
    /// hardcoded patterns — if one doesn't compile, we want tests to
    /// explode immediately, not limp along silently missing a rule.
    fn new(id: &str, name: &str, pattern: &str) -> Rule {
        Rule {
            id: id.to_string(),
            name: name.to_string(),
            pattern: Regex::new(pattern).expect("builtin pattern must compile"),
        }
    }
}

/// The default rule set shipped with Redactify.
pub fn builtin_rules() -> Vec<Rule> {
    vec![
        // local-part@domain.tld — deliberately pragmatic, not RFC 5322.
        // [A-Za-z0-9._%+-]+  : local part, common special chars
        // @[A-Za-z0-9.-]+    : domain labels and dots
        // \.[A-Za-z]{2,}     : requires a TLD, so "user@localhost" is NOT matched (documented choice)
        Rule::new(
            "email",
            "Email Address",
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b",
        ),
        // Four octets, each 0-255, dot-separated.
        // (?:...) is a non-capturing group — grouping without saving.
        // 25[0-5] | 2[0-4]\d | 1\d{2} | [1-9]?\d  covers 250-255, 200-249, 100-199, 0-99
        // so "999.999.999.999" is correctly rejected.
        Rule::new(
            "ipv4",
            "IPv4 Address",
            r"\b(?:(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)\b",
        ),
        // DECISION (documented): match liberally on shape (###-##-####),
        // don't validate SSA area-number rules. False positives are acceptable
        // because a human reviews findings — that's the whole product thesis.
        Rule::new("ssn", "US Social Security Number", r"\b\d{3}-\d{2}-\d{4}\b"),
        // US phone: optional +1 / 1 prefix, optional parens on area code,
        // separators may be dash, dot, or space.
        // \(?\d{3}\)? : escaped parens are literal "(" ")"; unescaped ones group.
        Rule::new(
            "us_phone",
            "US Phone Number",
            r"(?:\+?1[-. ]?)?(?:\(\d{3}\)|\b\d{3})[-. ]?\d{3}[-. ]?\d{4}\b",
        ),
        // AWS access key IDs: AKIA (long-term) or ASIA (temporary) + 16
        // uppercase alphanumerics. Fixed prefix makes this a HIGH-confidence
        // rule — the start of the "secrets, not just PII" story.
        Rule::new(
            "aws_access_key",
            "AWS Access Key ID",
            r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b",
        ),
    ]
}
