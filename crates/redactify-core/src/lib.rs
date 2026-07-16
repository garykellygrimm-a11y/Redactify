mod finding;
mod rules;

// Re-export so callers write `redactify_core::Rule`, not `redactify_core::rules::Rule`.
pub use finding::Finding;
pub use rules::{Rule, builtin_rules};

/// Scan `text` with `rules`, returning findings sorted by start offset,
/// with overlaps resolved (earliest start wins; on a tie, longest match wins).
pub fn detect(text: &str, rules: &[Rule]) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();

    for rule in rules {
        for m in rule.pattern.find_iter(text) {
            findings.push(Finding {
                start: m.start(),
                end: m.end(),
                // .clone() because this Finding must OWN its rule_id — the
                // rule keeps living its own life; we can't borrow from it
                // forever. Same story for matched: as_str() borrows from
                // `text`, to_string() copies it into something we own.
                rule_id: rule.id.clone(),
                matched: m.as_str().to_string(),
            });
        }
    }

    // Sort by start ascending; break ties by end DESCENDING, so when two
    // matches begin at the same offset, the longer one comes first and
    // therefore wins the overlap filter below.
    findings.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));

    // Overlap resolution in one pass: keep a finding only if it starts at or
    // after the end of the last finding we kept. Because of the sort order,
    // "first seen" is always the earliest-starting (then longest) claimant.
    let mut resolved: Vec<Finding> = Vec::new();
    for f in findings {
        match resolved.last() {
            Some(prev) if f.start < prev.end => {
                // Overlaps something we already kept — drop it on the floor.
            }
            _ => resolved.push(f),
        }
    }

    resolved
}

/// Replace each finding's span with `[REDACTED:{rule_id}]`.
/// Assumes `findings` is sorted and non-overlapping (detect guarantees this).
pub fn redact(text: &str, findings: &[Finding]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;

    for f in findings {
        // Copy the untouched text between the last finding and this one...
        out.push_str(&text[cursor..f.start]);
        // ...then the token instead of the sensitive span...
        out.push_str("[REDACTED:");
        out.push_str(&f.rule_id);
        out.push(']');
        // ...and jump the cursor past what we just replaced.
        cursor = f.end;
    }

    // Whatever remains after the final finding.
    out.push_str(&text[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- per-rule: true positive + near-miss negative ----------

    #[test]
    fn email_detects_and_rejects() {
        let rules = builtin_rules();
        let f = detect("contact bob@example.com today", &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "email");
        assert_eq!(f[0].matched, "bob@example.com");
        // near-miss: no TLD → not an email under our documented policy
        assert!(detect("user@localhost is exempt", &rules).is_empty());
    }

    #[test]
    fn ipv4_detects_and_rejects() {
        let rules = builtin_rules();
        let f = detect("server at 192.168.1.254 responded", &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "ipv4");
        assert_eq!(f[0].matched, "192.168.1.254");
        // near-miss: octet out of range
        assert!(detect("version 999.999.999.999 is fake", &rules).is_empty());
    }

    #[test]
    fn ssn_detects_and_rejects() {
        let rules = builtin_rules();
        let f = detect("SSN: 123-45-6789.", &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "ssn");
        // near-miss: wrong grouping (2-3-4)
        assert!(detect("order 12-345-6789 shipped", &rules).is_empty());
    }

    #[test]
    fn phone_detects() {
        let rules = builtin_rules();
        let f = detect("call (816) 555-0142 now", &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "us_phone");
        assert_eq!(f[0].matched, "(816) 555-0142");
    }

    #[test]
    fn aws_key_detects_and_rejects() {
        let rules = builtin_rules();
        let f = detect("key=AKIAIOSFODNN7EXAMPLE used", &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "aws_access_key");
        // near-miss: wrong prefix
        assert!(detect("key=BKIAIOSFODNN7EXAMPLE used", &rules).is_empty());
    }

    // ---------- the hard parts ----------

    #[test]
    fn findings_are_sorted_and_non_overlapping() {
        let rules = builtin_rules();
        // A phone number and an SSN-shaped token can overlap: 555-12-3456
        // matches ssn, and depending on your resolution, pieces of phone.
        // Whatever detect() returns must be sorted by start and never overlap.
        let text = "a 555-12-3456 b 10.0.0.1 c eve@corp.io";
        let f = detect(text, &rules);
        for w in f.windows(2) {
            assert!(w[0].start <= w[1].start, "findings must be sorted");
            assert!(w[0].end <= w[1].start, "findings must not overlap");
        }
    }

    #[test]
    fn redact_replaces_multiple_findings_exactly() {
        let rules = builtin_rules();
        let text = "bob@example.com logged in from 192.168.1.254";
        let out = redact(text, &detect(text, &rules));
        assert_eq!(out, "[REDACTED:email] logged in from [REDACTED:ipv4]");
    }

    #[test]
    fn redact_with_no_findings_returns_input_unchanged() {
        let out = redact("nothing sensitive here", &[]);
        assert_eq!(out, "nothing sensitive here");
    }

    #[test]
    fn redact_preserves_text_between_and_around_findings() {
        let rules = builtin_rules();
        let text = "start 1.2.3.4 middle 5.6.7.8 end";
        let out = redact(text, &detect(text, &rules));
        assert_eq!(out, "start [REDACTED:ipv4] middle [REDACTED:ipv4] end");
    }
}
