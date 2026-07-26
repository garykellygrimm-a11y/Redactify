mod error;
mod finding;
mod manifest;
mod rules;

pub use error::RedactifyError;
pub use finding::Finding;
pub use manifest::{Disposition, Manifest, ManifestFinding, sha256_hex};
pub use rules::{Rule, builtin_rules, load_rules_file, merge_rules};

/// Scan `text` with `rules`, returning findings sorted by start offset,
/// with overlaps resolved (earliest start wins; on a tie, longest match wins).
pub fn detect(text: &str, rules: &[Rule]) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();

    for rule in rules {
        for m in rule.pattern.find_iter(text) {
            // Only runs on strings the regex already matched — cost is
            // proportional to candidate count, not document size.
            if let Some(validate) = rule.validator
                && !validate(m.as_str())
            {
                continue;
            }
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

    #[test]
    fn gcp_api_key_detects_and_rejects() {
        let rules = builtin_rules();
        let text = "key: AIzaSyABCDEFGHIJKLMNOPQRSTUVWXYZabcd123 saved";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "gcp_api_key");
        assert_eq!(f[0].matched, "AIzaSyABCDEFGHIJKLMNOPQRSTUVWXYZabcd123");
        // near-miss: wrong prefix
        assert!(detect("key: BIzaSyABCDEFGHIJKLMNOPQRSTUVWXYZabcd123 saved", &rules).is_empty());
    }

    #[test]
    fn gcp_oauth_client_id_detects_and_rejects() {
        let rules = builtin_rules();
        let text =
            "client_id=123456789012-abc123def456ghi789jkl012mno345.apps.googleusercontent.com";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "gcp_oauth_client_id");
        // near-miss: right shape, wrong domain
        assert!(
            detect(
                "client_id=123456789012-abc123def456ghi789jkl012mno345.apps.example.com",
                &rules
            )
            .is_empty()
        );
    }

    #[test]
    fn oracle_ocid_detects_and_rejects() {
        let rules = builtin_rules();
        let text = "resource: ocid1.instance.oc1.iad.abuwcljtwfk7f5e2o3q6ircgpdty6rg52itdyg72tgdtbiwqlujt7vm5h3da here";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "oracle_ocid");
        // near-miss: wrong version literal ("ocid2" instead of "ocid1")
        assert!(detect(
            "resource: ocid2.instance.oc1.iad.abuwcljtwfk7f5e2o3q6ircgpdty6rg52itdyg72tgdtbiwqlujt7vm5h3da here",
            &rules
        )
        .is_empty());
    }

    #[test]
    fn oracle_ocid_allows_empty_region() {
        let rules = builtin_rules();
        // Tenancy OCIDs have no region segment (two consecutive dots).
        let f = detect(
            "tenancy: ocid1.tenancy.oc1..aaaaaaaazaizaakcbfd33qif7atm2a5vwppteukesf6dtyxpxgm66kvx3fmq",
            &rules,
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "oracle_ocid");
    }

    #[test]
    fn azure_sas_token_detects_and_rejects() {
        let rules = builtin_rules();
        let text = "token: sv=2015-04-05&st=2015-04-29T22:18:26Z&se=2015-04-30T02:23:26Z&sig=F6GRVAZ5Cdj2Pw4tgU7IlSTkWgn7bUkkAg8P6HESXwmf4B in logs";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "azure_sas_token");
        // near-miss: version present, but no signature parameter at all
        assert!(
            detect(
                "token: sv=2015-04-05&st=2015-04-29T22:18:26Z with no signature",
                &rules
            )
            .is_empty()
        );
    }

    #[test]
    fn private_key_block_detects_and_rejects() {
        let rules = builtin_rules();
        let f = detect("-----BEGIN RSA PRIVATE KEY-----\nMIIEow...", &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "private_key_block");
        // near-miss: a PEM block, but not a private key
        assert!(detect("-----BEGIN CERTIFICATE-----\nMIIDXT...", &rules).is_empty());
    }

    #[test]
    fn stripe_api_key_detects_and_rejects() {
        let rules = builtin_rules();
        let f = detect("key: sk_live_4eC39HqLyjWDarjtT1zdp7dc used", &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "stripe_api_key");
        // near-miss: "liv" instead of "live"
        assert!(detect("key: sk_liv_4eC39HqLyjWDarjtT1zdp7dc used", &rules).is_empty());
    }

    #[test]
    fn digitalocean_token_detects_and_rejects() {
        let rules = builtin_rules();
        let text =
            "token: dop_v1_60b49e2a8032f922d2001ea8a7b6c8ca63aefb197c3a0b83d0f588cfa8de1c8c used";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "digitalocean_token");
        // near-miss: suffix too short
        assert!(detect("token: dop_v1_60b49e2a used", &rules).is_empty());
    }

    #[test]
    fn sendgrid_api_key_detects_and_rejects() {
        let rules = builtin_rules();
        let text =
            "key: SG.aBcDeFgHiJkLmNoPqRsT12.zZyYxXwWvVuUtTsSrRqQpPoOnNmMlLkKjJiIhHgGfFe used";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "sendgrid_api_key");
        // near-miss: no second dot-separated segment
        assert!(detect("key: SG.notarealkeyformat used", &rules).is_empty());
    }

    #[test]
    fn github_token_detects_and_rejects() {
        let rules = builtin_rules();
        let f = detect(
            "token: ghp_16C7e42F292c6912E7710c838347Ae178B4a used",
            &rules,
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "github_token");
        // near-miss: suffix too short for either classic or fine-grained shape
        assert!(detect("token: ghp_short used", &rules).is_empty());
    }

    #[test]
    fn slack_token_detects_and_rejects() {
        let rules = builtin_rules();
        let text =
            "token: xoxb-96219857393-62330539414-22147117595-9d8cfc0f596f1ed002ab5595859014e used";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "slack_token");
        // near-miss: bare prefix with nothing after it (a real bug found
        // in another project's Slack-token regex, worth guarding against
        // explicitly rather than trusting the pattern by inspection)
        assert!(detect("just a bare xoxb- with nothing after", &rules).is_empty());
    }

    #[test]
    fn hashicorp_vault_token_detects_and_rejects() {
        let rules = builtin_rules();
        let text = "token: hvs.CAESILfkHZ292kPHfJlESXBMdWxsdnUabcdefghijklmno used";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "hashicorp_vault_token");
        // near-miss: suffix too short
        assert!(detect("token: hvs.tooshort used", &rules).is_empty());
    }

    #[test]
    fn ipv6_detects_common_forms() {
        let rules = builtin_rules();
        let f = detect("server at 2001:db8:85a3::8a2e:370:7334 responded", &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "ipv6");
        assert_eq!(f[0].matched, "2001:db8:85a3::8a2e:370:7334");

        let f2 = detect("loopback is ::1 here", &rules);
        assert_eq!(f2.len(), 1);
        assert_eq!(f2[0].rule_id, "ipv6");
        assert_eq!(f2[0].matched, "::1");
    }

    #[test]
    fn ipv6_rejects_time_and_ratio_like_pairs() {
        let rules = builtin_rules();
        // near-miss: a single colon (times, ratios) is not valid IPv6 —
        // every real form has at least two colons.
        assert!(detect("call me at 12:34 for lunch", &rules).is_empty());
        assert!(detect("score was 16:9 in the game", &rules).is_empty());
    }

    #[test]
    fn ipv6_rejects_hh_mm_ss_timestamps() {
        let rules = builtin_rules();
        // Regression test: an HH:MM:SS timestamp has exactly 2 colons
        // and all-digit (= valid hex) groups, which an earlier, more
        // liberal version of this rule accepted as a plausible short
        // IPv6 address — a real bug, caught via a Python prototype of
        // this same rule matching timestamps in log files during
        // manual review, not just a theoretical near-miss. A 2-3 group
        // address with single colons and no "::" was never valid IPv6
        // in the first place (compression via "::" is the only way to
        // have fewer than 8 groups), so this isn't a liberal-matching
        // trade-off — the earlier version was simply wrong.
        assert!(detect("2026-07-24 14:23:05 INFO server started", &rules).is_empty());
        assert!(detect("[14:23:05.123] request completed", &rules).is_empty());
        assert!(detect("timestamp: 09:15:42 UTC", &rules).is_empty());
        assert!(detect("duration was 00:05:30 total", &rules).is_empty());
    }

    #[test]
    fn ipv6_rejects_mac_addresses() {
        let rules = builtin_rules();
        // MAC addresses are the closest real-world look-alike: also
        // colon-separated hex, but always exactly 6 groups (not 8) and
        // never compressed with "::" — neither branch of the rule
        // should accept them.
        assert!(detect("MAC address: 00:1A:2B:3C:4D:5E on the network", &rules).is_empty());
    }

    #[test]
    fn openai_api_key_detects_and_rejects() {
        let rules = builtin_rules();
        let text = "key: sk-proj-ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij used";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "openai_api_key");
        // near-miss: legacy bare "sk-" with no documented sub-prefix is
        // deliberately excluded (see rules.rs comment)
        assert!(detect("key: sk-4eC39HqLyjWDarjtT1zdp7dcXXXXXXXX used", &rules).is_empty());
    }

    #[test]
    fn anthropic_api_key_detects_and_does_not_collide_with_openai_rule() {
        let rules = builtin_rules();
        let body = "B".repeat(80);
        let text = format!("key: sk-ant-api03-{body} used");
        let f = detect(&text, &rules);
        // exactly one finding: confirms the openai_api_key rule does NOT
        // also match this string, which it would if the OpenAI rule
        // accepted a bare "sk-" prefix instead of requiring a specific
        // sub-prefix.
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "anthropic_api_key");
    }

    #[test]
    fn npm_token_detects_and_rejects() {
        let rules = builtin_rules();
        let body = "c".repeat(36);
        let text = format!("token: npm_{body} used");
        let f = detect(&text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "npm_token");
        // near-miss: suffix too short
        assert!(detect("token: npm_tooshort used", &rules).is_empty());
    }

    #[test]
    fn twilio_sid_detects_and_rejects() {
        let rules = builtin_rules();
        let f = detect("sid: AC1234567890abcdef1234567890abcdef used", &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "twilio_sid");
        // near-miss: wrong prefix
        assert!(detect("sid: XX1234567890abcdef1234567890abcdef used", &rules).is_empty());
    }

    #[test]
    fn credit_card_validates_via_luhn_not_shape_alone() {
        let rules = builtin_rules();
        // Standard, publicly-documented test numbers (Visa/Mastercard/Amex
        // test values used throughout the payments industry) — not real
        // cards. All three lengths (16/16/15 digits) confirm the rule
        // isn't hardcoded to one card length.
        let f = detect("card: 4111111111111111 on file", &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "credit_card");

        assert_eq!(
            detect("mastercard 5500005555555559 charged", &rules).len(),
            1
        );
        assert_eq!(detect("amex: 378282246310005 stored", &rules).len(), 1);

        // Grouped with separators — the regex is deliberately liberal
        // about this, since Luhn (not the shape) does the real filtering.
        let grouped = detect("card: 4111-1111-1111-1111 on file", &rules);
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].matched, "4111-1111-1111-1111");

        // near-miss: same length and digit-run shape as a real card, but
        // fails the Luhn checksum — this is the whole point of adding a
        // validator instead of matching any 16-digit run.
        assert!(detect("order number 1234567890123456 today", &rules).is_empty());
    }

    #[test]
    fn jwt_validates_header_not_shape_alone() {
        let rules = builtin_rules();
        // A real, publicly-documented example JWT (jwt.io's own
        // introductory example) — not a live token.
        let text = "token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c used";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "jwt");

        // near-miss: three dot-separated segments (matches the bare
        // shape) that are NOT a JWT — a dotted package/module name. This
        // is exactly the false-positive category a shape-only rule would
        // have caught, and the validator correctly rejects.
        assert!(detect("import com.example.app.MainActivity", &rules).is_empty());
    }

    #[test]
    fn bitcoin_address_validates_checksum_not_shape_alone() {
        let rules = builtin_rules();
        // Real, publicly-known addresses: the Bitcoin genesis block
        // address (P2PKH, starts with 1) and a well-known P2SH address
        // (starts with 3) — both real, neither a secret to expose.
        let f = detect("wallet: 1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa used", &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "bitcoin_address");

        assert_eq!(
            detect("p2sh: 3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy used", &rules).len(),
            1
        );

        // near-miss: same shape and length, but the last character is
        // altered — fails the Base58Check checksum.
        assert!(detect("wallet: 1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb used", &rules).is_empty());
    }

    #[test]
    fn discord_webhook_detects_and_rejects() {
        let rules = builtin_rules();
        let text = "hook: https://discord.com/api/webhooks/123456789123456789/C9WPqExYWONPDZabcdef-def1434FGFjstasJX9pYht73y used";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "discord_webhook");
        // near-miss: a discord.com URL that isn't a webhook endpoint
        assert!(detect("link: https://discord.com/channels/123/456 used", &rules).is_empty());
    }

    #[test]
    fn mailchimp_api_key_detects_and_rejects() {
        let rules = builtin_rules();
        let text = "key: abc123def456abc123def456abc123de-us14 used";
        let f = detect(text, &rules);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule_id, "mailchimp_api_key");
        // near-miss: the 32-char hex key with no datacenter suffix at
        // all — not a usable Mailchimp key, and correctly not flagged.
        assert!(
            detect("key: abc123def456abc123def456abc123de used", &rules).is_empty()
        );
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
