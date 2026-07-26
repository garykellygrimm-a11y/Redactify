use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RedactifyError;
use crate::finding::Finding;
use crate::rules::Rule;

/// What became of a finding: applied to the output, or explicitly
/// declined by a human reviewer. In unreviewed contexts (the CLI),
/// every finding is Accepted — no human was in the loop to reject.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Disposition {
    Accepted,
    Rejected,
}

/// One redaction event in the audit record.
///
/// Deliberately content-free: no matched text, no per-finding hash.
/// Rejected findings ARE recorded — "a reviewer saw this and declined
/// it" is audit evidence, not leakage; the manifest still reveals
/// nothing without the original file. See docs/adr/001.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestFinding {
    pub rule_id: String,
    pub start: usize,
    pub end: usize,
    pub length: usize,
    pub disposition: Disposition,
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
    /// All findings detected, applied or not.
    pub finding_count: usize,
    /// How many findings were actually redacted in the output.
    pub applied_count: usize,
    pub findings: Vec<ManifestFinding>,
}

/// Hex-encoded SHA-256 digest of `data`.
pub fn sha256_hex(data: &str) -> String {
    hex::encode(Sha256::digest(data.as_bytes()))
}

impl Manifest {
    /// Build a manifest describing one redaction run.
    ///
    /// `dispositions` must be the same length and order as `findings`.
    /// For unreviewed runs, pass all-Accepted (see [`Manifest::unreviewed`]).
    pub fn new(
        tool: &str,
        source: &str,
        output: &str,
        rules: &[Rule],
        findings: &[Finding],
        dispositions: &[Disposition],
    ) -> Manifest {
        assert_eq!(
            findings.len(),
            dispositions.len(),
            "one disposition per finding"
        );
        let applied_count = dispositions
            .iter()
            .filter(|d| **d == Disposition::Accepted)
            .count();
        Manifest {
            tool: tool.to_string(),
            created_utc: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            source_sha256: sha256_hex(source),
            output_sha256: sha256_hex(output),
            rules_applied: rules.iter().map(|r| r.id.clone()).collect(),
            finding_count: findings.len(),
            applied_count,
            findings: findings
                .iter()
                .zip(dispositions)
                .map(|(f, d)| ManifestFinding {
                    rule_id: f.rule_id.clone(),
                    start: f.start,
                    end: f.end,
                    length: f.end - f.start,
                    disposition: *d,
                })
                .collect(),
        }
    }

    /// Manifest for a run with no human review: every finding Accepted.
    pub fn unreviewed(
        tool: &str,
        source: &str,
        output: &str,
        rules: &[Rule],
        findings: &[Finding],
    ) -> Manifest {
        let dispositions = vec![Disposition::Accepted; findings.len()];
        Manifest::new(tool, source, output, rules, findings, &dispositions)
    }

    /// Serialize to pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, RedactifyError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parse a manifest back from JSON (the inverse of [`Manifest::to_json`]).
    pub fn from_json(json: &str) -> Result<Manifest, RedactifyError> {
        Ok(serde_json::from_str(json)?)
    }

    /// Check that `source` and `output` are consistent with this
    /// manifest's account of what happened. See [`VerifyReport`] for what
    /// each individual check means.
    pub fn verify(&self, source: &str, output: &str) -> VerifyReport {
        let source_hash_matches = sha256_hex(source) == self.source_sha256;
        let output_hash_matches = sha256_hex(output) == self.output_sha256;

        // redact() only ever reads start/end/rule_id (never `matched`),
        // and it's a pure function: same source + same accepted findings
        // always produces exactly the same output. So regenerating the
        // output from `source` and just this manifest's own accepted
        // findings, then comparing byte-for-byte against the given
        // `output`, proves three things in one check rather than three
        // separate ones: every accepted finding was actually redacted,
        // every rejected finding survived untouched, and nothing else in
        // the document was altered — any one of those being false would
        // make the reconstruction disagree with the real output.
        let accepted: Vec<Finding> = self
            .findings
            .iter()
            .filter(|f| f.disposition == Disposition::Accepted)
            .map(|f| Finding {
                start: f.start,
                end: f.end,
                rule_id: f.rule_id.clone(),
                matched: String::new(),
            })
            .collect();
        let reconstructed = crate::redact(source, &accepted);
        let redaction_matches = reconstructed == output;

        VerifyReport {
            source_hash_matches,
            output_hash_matches,
            redaction_matches,
        }
    }
}

/// Outcome of [`Manifest::verify`]. Each check is independent and
/// reported separately, rather than collapsed into one bool, so a
/// caller (the CLI, or anything else) can tell a reviewer exactly what
/// did or didn't line up instead of just "verification failed."
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerifyReport {
    /// Does `sha256_hex(source)` match `manifest.source_sha256`? False
    /// usually means the wrong original file was provided, or it was
    /// edited since the manifest was created.
    pub source_hash_matches: bool,
    /// Does `sha256_hex(output)` match `manifest.output_sha256`? False
    /// usually means the wrong output file was provided, or it was
    /// altered after export.
    pub output_hash_matches: bool,
    /// Does regenerating the output from `source` + the manifest's own
    /// accepted findings reproduce the given `output` byte-for-byte?
    /// This is the check that actually proves every finding is
    /// accounted for, not just that the files' hashes agree with
    /// whatever they happen to contain.
    pub redaction_matches: bool,
}

impl VerifyReport {
    /// True only if every individual check passed.
    pub fn all_passed(&self) -> bool {
        self.source_hash_matches && self.output_hash_matches && self.redaction_matches
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
        let m = Manifest::unreviewed("redactify test", text, &output, &rules, &findings);

        assert_eq!(m.finding_count, 2);
        assert_eq!(m.applied_count, 2);
        assert_eq!(m.findings[0].rule_id, "email");
        assert_eq!(m.findings[0].length, "bob@example.com".len());
        assert_eq!(m.findings[0].disposition, Disposition::Accepted);
        assert_eq!(m.source_sha256, sha256_hex(text));
        assert_eq!(m.output_sha256, sha256_hex(&output));
        assert_eq!(m.rules_applied.len(), 30);
    }

    #[test]
    fn reviewed_manifest_records_rejections() {
        let rules = builtin_rules();
        let text = "bob@example.com and 10.0.0.1";
        let findings = detect(text, &rules);
        assert_eq!(findings.len(), 2);

        // Reviewer accepts the email, rejects the IP.
        let dispositions = [Disposition::Accepted, Disposition::Rejected];
        let accepted: Vec<_> = findings
            .iter()
            .zip(&dispositions)
            .filter(|(_, d)| **d == Disposition::Accepted)
            .map(|(f, _)| f.clone())
            .collect();
        let output = redact(text, &accepted);

        let m = Manifest::new(
            "redactify test",
            text,
            &output,
            &rules,
            &findings,
            &dispositions,
        );
        assert_eq!(m.finding_count, 2);
        assert_eq!(m.applied_count, 1);
        assert_eq!(m.findings[1].disposition, Disposition::Rejected);
        // The rejected IP survives in the output; the email does not.
        assert!(output.contains("10.0.0.1"));
        assert!(!output.contains("bob@example.com"));
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let rules = builtin_rules();
        let text = "SSN 123-45-6789 on file";
        let findings = detect(text, &rules);
        let output = redact(text, &findings);
        let original = Manifest::unreviewed("redactify test", text, &output, &rules, &findings);

        let json = original.to_json().expect("serialization");
        let parsed: Manifest = serde_json::from_str(&json).expect("deserialization");
        assert_eq!(original, parsed);
    }

    #[test]
    fn manifest_contains_no_matched_content() {
        // The ADR promise, executable: sensitive text must never appear
        // in the serialized manifest — including for rejected findings.
        let rules = builtin_rules();
        let text = "leak: AKIAIOSFODNN7EXAMPLE and bob@example.com";
        let findings = detect(text, &rules);
        let dispositions = [Disposition::Accepted, Disposition::Rejected];
        let accepted: Vec<_> = findings
            .iter()
            .zip(&dispositions)
            .filter(|(_, d)| **d == Disposition::Accepted)
            .map(|(f, _)| f.clone())
            .collect();
        let output = redact(text, &accepted);
        let json = Manifest::new(
            "redactify test",
            text,
            &output,
            &rules,
            &findings,
            &dispositions,
        )
        .to_json()
        .expect("serialization");

        assert!(!json.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!json.contains("bob@example.com"));
    }

    #[test]
    fn verify_passes_for_a_correctly_redacted_output() {
        let rules = builtin_rules();
        let text = "bob@example.com and 10.0.0.1 both appear here";
        let findings = detect(text, &rules);
        let dispositions = [Disposition::Accepted, Disposition::Rejected];
        let accepted: Vec<_> = findings
            .iter()
            .zip(&dispositions)
            .filter(|(_, d)| **d == Disposition::Accepted)
            .map(|(f, _)| f.clone())
            .collect();
        let output = redact(text, &accepted);
        let manifest = Manifest::new(
            "redactify test",
            text,
            &output,
            &rules,
            &findings,
            &dispositions,
        );

        let report = manifest.verify(text, &output);
        assert!(report.all_passed());
        assert!(report.source_hash_matches);
        assert!(report.output_hash_matches);
        assert!(report.redaction_matches);
    }

    #[test]
    fn verify_catches_wrong_source_file() {
        let rules = builtin_rules();
        let text = "bob@example.com on file";
        let findings = detect(text, &rules);
        let output = redact(text, &findings);
        let manifest = Manifest::unreviewed("redactify test", text, &output, &rules, &findings);

        let report = manifest.verify("this is not the original file at all", &output);
        assert!(!report.source_hash_matches);
        assert!(!report.all_passed());
    }

    #[test]
    fn verify_catches_tampered_output() {
        let rules = builtin_rules();
        let text = "bob@example.com on file";
        let findings = detect(text, &rules);
        let output = redact(text, &findings);
        let manifest = Manifest::unreviewed("redactify test", text, &output, &rules, &findings);

        let tampered_output = format!("{output} plus some appended text");
        let report = manifest.verify(text, &tampered_output);
        assert!(!report.output_hash_matches);
        assert!(!report.redaction_matches);
        assert!(!report.all_passed());
    }

    #[test]
    fn verify_catches_a_disposition_edited_after_the_fact() {
        // The scenario the hash checks alone CANNOT catch: the output
        // file is untouched (so its hash still matches), but someone
        // edited the manifest JSON itself to claim a finding was
        // rejected when it was actually redacted — or vice versa. Only
        // the reconstruction check notices this, because it rebuilds
        // from the recorded dispositions and compares against the real
        // output, rather than trusting the recorded hash alone.
        let rules = builtin_rules();
        let text = "bob@example.com and 10.0.0.1 both appear here";
        let findings = detect(text, &rules);
        let dispositions = [Disposition::Accepted, Disposition::Rejected];
        let accepted: Vec<_> = findings
            .iter()
            .zip(&dispositions)
            .filter(|(_, d)| **d == Disposition::Accepted)
            .map(|(f, _)| f.clone())
            .collect();
        let output = redact(text, &accepted);
        let mut manifest = Manifest::new(
            "redactify test",
            text,
            &output,
            &rules,
            &findings,
            &dispositions,
        );

        // Flip the rejected IP to "Accepted" in the manifest, without
        // touching the actual output file at all.
        manifest.findings[1].disposition = Disposition::Accepted;

        let report = manifest.verify(text, &output);
        // Neither hash changed, since neither input file changed...
        assert!(report.source_hash_matches);
        assert!(report.output_hash_matches);
        // ...but reconstructing from the (tampered) manifest now expects
        // the IP to be redacted too, which the real output never did.
        assert!(!report.redaction_matches);
        assert!(!report.all_passed());
    }

    #[test]
    fn manifest_json_round_trips_via_from_json() {
        let rules = builtin_rules();
        let text = "SSN 123-45-6789 on file";
        let findings = detect(text, &rules);
        let output = redact(text, &findings);
        let original = Manifest::unreviewed("redactify test", text, &output, &rules, &findings);

        let json = original.to_json().expect("serialization");
        let parsed = Manifest::from_json(&json).expect("from_json");
        assert_eq!(original, parsed);
    }
}
