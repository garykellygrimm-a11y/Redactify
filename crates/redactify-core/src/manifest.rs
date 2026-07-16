use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RedactifyError;
use crate::finding::Finding;
use crate::rules::Rule;

/// One redaction event in the audit record.
///
/// Deliberately content-free: no matched text, no per-finding hash.
/// See docs/adr/001-manifest-content.md for the reasoning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestFinding {
    pub rule_id: String,
    pub start: usize,
    pub end: usize,
    pub length: usize,
}

/// Audit record for a single redaction operation.
///
/// Integrity lives at the document level: SHA-256 of the full source and
/// full output. Anyone holding the original can verify the entire chain;
/// anyone holding only the manifest learns nothing sensitive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Tool name and version that produced this manifest.
    pub tool: String,
    /// RFC 3339 UTC timestamp of the operation.
    pub created_utc: String,
    pub source_sha256: String,
    pub output_sha256: String,
    /// Ids of every rule that was active (not just ones that matched).
    pub rules_applied: Vec<String>,
    pub finding_count: usize,
    pub findings: Vec<ManifestFinding>,
}

/// Hex-encoded SHA-256 digest of `data`.
pub fn sha256_hex(data: &str) -> String {
    hex::encode(Sha256::digest(data.as_bytes()))
}

impl Manifest {
    /// Build a manifest describing one redaction run.
    pub fn new(
        tool: &str,
        source: &str,
        output: &str,
        rules: &[Rule],
        findings: &[Finding],
    ) -> Manifest {
        Manifest {
            tool: tool.to_string(),
            created_utc: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            source_sha256: sha256_hex(source),
            output_sha256: sha256_hex(output),
            rules_applied: rules.iter().map(|r| r.id.clone()).collect(),
            finding_count: findings.len(),
            findings: findings
                .iter()
                .map(|f| ManifestFinding {
                    rule_id: f.rule_id.clone(),
                    start: f.start,
                    end: f.end,
                    length: f.end - f.start,
                })
                .collect(),
        }
    }

    /// Serialize to pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, RedactifyError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builtin_rules, detect, redact};

    #[test]
    fn sha256_matches_known_test_vector() {
        // Published NIST test vector for "abc".
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn manifest_captures_run_accurately() {
        let rules = builtin_rules();
        let text = "bob@example.com logged in from 192.168.1.254";
        let findings = detect(text, &rules);
        let output = redact(text, &findings);
        let m = Manifest::new("redactify test", text, &output, &rules, &findings);

        assert_eq!(m.finding_count, 2);
        assert_eq!(m.findings[0].rule_id, "email");
        assert_eq!(m.findings[0].length, "bob@example.com".len());
        assert_eq!(m.source_sha256, sha256_hex(text));
        assert_eq!(m.output_sha256, sha256_hex(&output));
        assert_eq!(m.rules_applied.len(), 5);
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let rules = builtin_rules();
        let text = "SSN 123-45-6789 on file";
        let findings = detect(text, &rules);
        let output = redact(text, &findings);
        let original = Manifest::new("redactify test", text, &output, &rules, &findings);

        let json = original.to_json().expect("serialization");
        let parsed: Manifest = serde_json::from_str(&json).expect("deserialization");
        assert_eq!(original, parsed);
    }

    #[test]
    fn manifest_contains_no_matched_content() {
        // The ADR promise, executable: sensitive text must never appear
        // in the serialized manifest.
        let rules = builtin_rules();
        let text = "leak: AKIAIOSFODNN7EXAMPLE and bob@example.com";
        let findings = detect(text, &rules);
        let output = redact(text, &findings);
        let json = Manifest::new("redactify test", text, &output, &rules, &findings)
            .to_json()
            .expect("serialization");

        assert!(!json.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!json.contains("bob@example.com"));
    }
}
