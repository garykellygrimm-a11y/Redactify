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
    /// Convenience constructor for builtins. `.expect()` is deliberate:
    /// these are OUR hardcoded patterns — if one doesn't compile, we want
    /// tests to explode immediately, not limp along silently missing a rule.
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
    ]
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
        assert_eq!(merged.len(), 5, "override must replace, not add");
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
        assert_eq!(merged.len(), 6);
    }
}
