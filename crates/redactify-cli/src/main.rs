use std::collections::BTreeMap;
use std::process::ExitCode;

// Dash in the crate name becomes an underscore in `use` — cargo quirk.
use redactify_core::{builtin_rules, detect, redact};

fn main() -> ExitCode {
    // args().nth(0) is the program name itself; nth(1) is the first real arg.
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: redactify <file>");
        return ExitCode::FAILURE;
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(e) => {
            eprintln!("error: could not read '{path}': {e}");
            return ExitCode::FAILURE;
        }
    };

    let rules = builtin_rules();
    let findings = detect(&text, &rules);

    // Redacted content -> stdout, so `redactify app.log > clean.log` works.
    print!("{}", redact(&text, &findings));

    // Summary -> stderr, so it's visible in the terminal but never
    // contaminates redirected output.
    if findings.is_empty() {
        eprintln!("0 findings");
    } else {
        // BTreeMap instead of HashMap for deterministic (alphabetical)
        // ordering — stable output is testable output.
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for f in &findings {
            *counts.entry(f.rule_id.as_str()).or_insert(0) += 1;
        }
        let breakdown: Vec<String> = counts
            .iter()
            .map(|(rule_id, n)| format!("{n} {rule_id}"))
            .collect();
        eprintln!("{} findings: {}", findings.len(), breakdown.join(", "));
    }

    ExitCode::SUCCESS
}
