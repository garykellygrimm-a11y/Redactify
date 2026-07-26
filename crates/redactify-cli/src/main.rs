use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use redactify_core::{Manifest, builtin_rules, detect, load_rules_file, merge_rules, redact};

#[derive(Parser)]
#[command(name = "redactify", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan a file for sensitive data and produce sanitized output.
    Scan(ScanArgs),
    /// Check that an original file, a redacted output, and a manifest are
    /// mutually consistent with each other.
    Verify(VerifyArgs),
}

#[derive(clap::Args)]
struct ScanArgs {
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

#[derive(clap::Args)]
struct VerifyArgs {
    /// The original, unredacted file
    original: PathBuf,

    /// The redacted output file to check against the manifest
    #[arg(long)]
    output: PathBuf,

    /// The JSON audit manifest produced when the output was created
    #[arg(long)]
    manifest: PathBuf,
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Scan(args) => match run_scan(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Command::Verify(args) => match run_verify(args) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
    }
}

fn run_scan(cli: ScanArgs) -> Result<(), Box<dyn std::error::Error>> {
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
        let manifest = Manifest::unreviewed(&tool, &text, &redacted, &rules, &findings);
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

/// Runs `verify`. Returns Ok(true)/Ok(false) for "ran fine, here's the
/// verdict" rather than treating a failed verification as an Err — a
/// manifest not checking out is an expected, reportable outcome, not a
/// program error. Err is reserved for things like unreadable files or
/// unparseable JSON.
fn run_verify(args: VerifyArgs) -> Result<bool, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(&args.original)
        .map_err(|e| format!("could not read '{}': {e}", args.original.display()))?;
    let output = fs::read_to_string(&args.output)
        .map_err(|e| format!("could not read '{}': {e}", args.output.display()))?;
    let manifest_json = fs::read_to_string(&args.manifest)
        .map_err(|e| format!("could not read '{}': {e}", args.manifest.display()))?;
    let manifest = Manifest::from_json(&manifest_json)
        .map_err(|e| format!("invalid manifest '{}': {e}", args.manifest.display()))?;

    let report = manifest.verify(&source, &output);

    println!(
        "{} source file matches manifest",
        if report.source_hash_matches {
            "\u{2713}"
        } else {
            "\u{2717}"
        }
    );
    println!(
        "{} output file matches manifest",
        if report.output_hash_matches {
            "\u{2713}"
        } else {
            "\u{2717}"
        }
    );
    println!(
        "{} every finding is accounted for (redaction reconstructs exactly)",
        if report.redaction_matches {
            "\u{2713}"
        } else {
            "\u{2717}"
        }
    );

    if report.all_passed() {
        println!("\nverified: this manifest's account of what happened checks out.");
    } else {
        println!("\nverification FAILED: the manifest does not match the given files.");
    }

    Ok(report.all_passed())
}
