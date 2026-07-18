use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use redactify_core::{builtin_rules, detect, load_rules_file, merge_rules, redact, Manifest};

/// Scan a file for sensitive data and produce sanitized output.
#[derive(Parser)]
#[command(name = "redactify", version, about)]
struct Cli {
    /// File to scan
    input: PathBuf,

    /// Write redacted output to a file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Write a JSON audit manifest to this path
    #[arg(long)]
    manifest: Option<PathBuf>,

    /// Load additional rules from a TOML file (same-id rules override builtins)
    #[arg(long)]
    rules: Option<PathBuf>,

    /// Disable builtin rules; scan with --rules patterns only
    #[arg(long, requires = "rules")]
    no_builtins: bool,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let text = fs::read_to_string(&cli.input)
        .map_err(|e| format!("could not read '{}': {e}", cli.input.display()))?;

    let user_rules = match &cli.rules {
        Some(path) => load_rules_file(path)?,
        None => Vec::new(),
    };
    let rules = if cli.no_builtins {
        user_rules
    } else {
        merge_rules(builtin_rules(), user_rules)
    };
    if rules.is_empty() {
        return Err("no rules to apply (rules file is empty and builtins are disabled)".into());
    }

    let findings = detect(&text, &rules);
    let redacted = redact(&text, &findings);

    // Redacted content: file if -o given, otherwise stdout (pipe-friendly).
    match &cli.output {
        Some(path) => fs::write(path, &redacted)
            .map_err(|e| format!("could not write '{}': {e}", path.display()))?,
        None => print!("{redacted}"),
    }

    if let Some(path) = &cli.manifest {
        let tool = format!("redactify {}", env!("CARGO_PKG_VERSION"));
        let manifest = Manifest::new(&tool, &text, &redacted, &rules, &findings);
        fs::write(path, manifest.to_json()?)
            .map_err(|e| format!("could not write manifest '{}': {e}", path.display()))?;
    }

    // Summary -> stderr, same contract as before.
    if findings.is_empty() {
        eprintln!("0 findings");
    } else {
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

    Ok(())
}
