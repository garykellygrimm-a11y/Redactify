use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use rayon::prelude::*;
use redactify_core::{Manifest, Rule, builtin_rules, detect, load_rules_file, merge_rules, redact};
use walkdir::WalkDir;

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
    /// Scan many files (and/or whole directories) in one run.
    Batch(BatchArgs),
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

#[derive(clap::Args)]
struct BatchArgs {
    /// One or more files and/or directories to scan
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Recurse into subdirectories when a directory is given (default:
    /// only files directly inside it)
    #[arg(short = 'r', long)]
    recursive: bool,

    /// Write redacted output (and a manifest alongside each, following
    /// the same <output>.manifest.json convention the desktop app uses)
    /// into this directory. Each input's relative structure is mirrored
    /// into it, and files from different top-level directory arguments
    /// are nested under a folder named for that directory, so files with
    /// the same name in different source directories never collide.
    /// Omit to scan-only and just report findings per file.
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Load additional rules from a TOML file (same-id rules override
    /// builtins). Applied uniformly across every file in the batch.
    #[arg(long)]
    rules: Option<PathBuf>,

    /// Disable builtin rules; scan with --rules patterns only
    #[arg(long, requires = "rules")]
    no_builtins: bool,

    /// Limit parallelism to this many threads (default: number of CPU cores)
    #[arg(short = 'j', long)]
    jobs: Option<usize>,
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
        Command::Batch(args) => match run_batch(args) {
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

/// One file resolved out of the batch's input list: where to read it
/// from, and (if writing output) where its output belongs relative to
/// --output-dir.
struct BatchItem {
    input_path: PathBuf,
    output_rel: PathBuf,
}

/// Expand `inputs` (a mix of files and directories) into a flat list of
/// files to process. Directory entries get their relative path (from
/// that directory) nested under a folder named for that directory's own
/// basename, so two directories that happen to contain same-named files
/// never map to the same output path. Bare file inputs map directly to
/// their own filename at the output root.
fn expand_batch_inputs(
    inputs: &[PathBuf],
    recursive: bool,
) -> Result<Vec<BatchItem>, Box<dyn std::error::Error>> {
    let mut items = Vec::new();

    for input in inputs {
        if input.is_dir() {
            let base_name = input
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("_"));

            let mut walker = WalkDir::new(input);
            if !recursive {
                walker = walker.max_depth(1);
            }
            for entry in walker {
                let entry =
                    entry.map_err(|e| format!("could not walk '{}': {e}", input.display()))?;
                if entry.file_type().is_file() {
                    let rel = entry.path().strip_prefix(input)?;
                    items.push(BatchItem {
                        input_path: entry.path().to_path_buf(),
                        output_rel: base_name.join(rel),
                    });
                }
            }
        } else if input.is_file() {
            let filename = input
                .file_name()
                .ok_or_else(|| format!("'{}' has no filename", input.display()))?;
            items.push(BatchItem {
                input_path: input.clone(),
                output_rel: PathBuf::from(filename),
            });
        } else {
            return Err(format!("'{}' is not a file or directory", input.display()).into());
        }
    }

    Ok(items)
}

enum BatchOutcome {
    Ok {
        input: PathBuf,
        finding_count: usize,
    },
    Err {
        input: PathBuf,
        message: String,
    },
}

fn run_batch(cli: BatchArgs) -> Result<bool, Box<dyn std::error::Error>> {
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

    let items = expand_batch_inputs(&cli.inputs, cli.recursive)?;
    if items.is_empty() {
        return Err("no files found to process".into());
    }

    // Detect output-path collisions across different top-level inputs up
    // front, rather than silently letting one file overwrite another.
    let mut seen: HashSet<&Path> = HashSet::new();
    let mut collisions: HashSet<PathBuf> = HashSet::new();
    for item in &items {
        if !seen.insert(item.output_rel.as_path()) {
            collisions.insert(item.output_rel.clone());
        }
    }

    if let Some(jobs) = cli.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build_global()
            .map_err(|e| format!("could not configure thread pool: {e}"))?;
    }

    let tool = format!("redactify {}", env!("CARGO_PKG_VERSION"));

    let results: Vec<BatchOutcome> = items
        .par_iter()
        .map(|item| {
            if collisions.contains(&item.output_rel) {
                return BatchOutcome::Err {
                    input: item.input_path.clone(),
                    message: format!(
                        "output path '{}' collides with another input; skipped",
                        item.output_rel.display()
                    ),
                };
            }
            match process_one(item, &rules, &cli.output_dir, &tool) {
                Ok(finding_count) => BatchOutcome::Ok {
                    input: item.input_path.clone(),
                    finding_count,
                },
                Err(e) => BatchOutcome::Err {
                    input: item.input_path.clone(),
                    message: e.to_string(),
                },
            }
        })
        .collect();

    let mut ok_count = 0usize;
    let mut err_count = 0usize;
    for r in &results {
        match r {
            BatchOutcome::Ok {
                input,
                finding_count,
            } => {
                ok_count += 1;
                eprintln!("{}: {finding_count} findings", input.display());
            }
            BatchOutcome::Err { input, message } => {
                err_count += 1;
                eprintln!("{}: ERROR: {message}", input.display());
            }
        }
    }
    eprintln!(
        "\n{} file(s) processed: {ok_count} succeeded, {err_count} failed",
        results.len()
    );

    Ok(err_count == 0)
}

/// Scan and (if `output_dir` is set) redact one batch item. Returns the
/// finding count on success.
fn process_one(
    item: &BatchItem,
    rules: &[Rule],
    output_dir: &Option<PathBuf>,
    tool: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(&item.input_path).map_err(|e| format!("could not read: {e}"))?;
    let findings = detect(&text, rules);
    let redacted = redact(&text, &findings);

    if let Some(dir) = output_dir {
        let out_path = dir.join(&item.output_rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("could not create output directory: {e}"))?;
        }
        fs::write(&out_path, &redacted).map_err(|e| format!("could not write output: {e}"))?;

        // Same <output>.manifest.json convention the desktop app's
        // export uses, for consistency across the product.
        let manifest_path = PathBuf::from(format!("{}.manifest.json", out_path.display()));
        let manifest = Manifest::unreviewed(tool, &text, &redacted, rules, &findings);
        let json = manifest
            .to_json()
            .map_err(|e| format!("could not serialize manifest: {e}"))?;
        fs::write(&manifest_path, json).map_err(|e| format!("could not write manifest: {e}"))?;
    }

    Ok(findings.len())
}
