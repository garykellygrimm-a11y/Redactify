//! Corpus scan: run the builtin rules across a directory of real-world
//! files and report where they fire.
//!
//! This exists because per-rule unit tests can't answer the question that
//! actually matters — "what does this rule do to a real log file?" A rule
//! with a clean true-positive test and a clean near-miss test can still be
//! useless in practice if it fires on every timestamp in a syslog.
//!
//! Deliberately an example rather than a test: it needs a corpus that
//! isn't (and shouldn't be) committed to this repo, and its output is a
//! report to read rather than an assertion to pass. Output is
//! deterministic so two runs can be diffed to measure whether a rule
//! change actually helped.
//!
//! Usage:
//!   cargo run --release -p redactify-core --example corpus_scan -- <path>
//!   cargo run --release -p redactify-core --example corpus_scan -- <path> --samples 5 --ext log,txt
//!
//! Run it with --release. The regex engine is dramatically slower in a
//! debug build, and a corpus scan is exactly the workload that shows it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use redactify_core::{builtin_rules, detect};

struct Args {
    root: PathBuf,
    samples: usize,
    exts: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args().skip(1);
    let root = args
        .next()
        .ok_or("usage: corpus_scan <path> [--samples N] [--ext log,txt]")?;
    let mut samples = 3usize;
    // Default to log-shaped files. Pointing this at a loghub checkout
    // would otherwise also pick up the *_structured.csv files, which
    // restate the same log lines in another form and would double-count
    // every finding.
    let mut exts = vec!["log".to_string(), "txt".to_string()];

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--samples" => {
                samples = args
                    .next()
                    .ok_or("--samples needs a number")?
                    .parse()
                    .map_err(|_| "--samples needs a number".to_string())?;
            }
            "--ext" => {
                exts = args
                    .next()
                    .ok_or("--ext needs a comma-separated list")?
                    .split(',')
                    .map(|e| e.trim().trim_start_matches('.').to_lowercase())
                    .filter(|e| !e.is_empty())
                    .collect();
            }
            // "*" means don't filter at all, for corpora with no extensions.
            "--all" => exts.clear(),
            other => return Err(format!("unknown option '{other}'")),
        }
    }

    Ok(Args {
        root: PathBuf::from(root),
        samples,
        exts,
    })
}

/// Recursive walk, kept to std rather than pulling walkdir in as a
/// dev-dependency for one function.
fn collect_files(dir: &Path, exts: &[String], out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(&path, exts, out)?;
        } else if exts.is_empty() {
            out.push(path);
        } else {
            let matches = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| exts.contains(&e.to_lowercase()))
                .unwrap_or(false);
            if matches {
                out.push(path);
            }
        }
    }
    Ok(())
}

/// Byte offsets where each line begins, for turning a finding's offset
/// into a line number without rescanning the text per finding.
fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    starts
}

/// 1-based line number containing `offset`.
fn line_of(starts: &[usize], offset: usize) -> usize {
    match starts.binary_search(&offset) {
        Ok(i) => i + 1,
        Err(i) => i,
    }
}

fn line_text<'a>(text: &'a str, starts: &[usize], line: usize) -> &'a str {
    let begin = starts[line - 1];
    let end = starts.get(line).copied().unwrap_or(text.len());
    text[begin..end].trim_end()
}

struct Sample {
    file: String,
    line: usize,
    matched: String,
    context: String,
}

fn commas(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let kept: String = s.chars().take(max).collect();
        format!("{kept}…")
    }
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut files = Vec::new();
    if args.root.is_dir() {
        if let Err(e) = collect_files(&args.root, &args.exts, &mut files) {
            eprintln!("error walking '{}': {e}", args.root.display());
            return ExitCode::FAILURE;
        }
    } else {
        // Single-file mode, so this can be pointed at one large generated
        // log without wrapping it in a directory.
        files.push(args.root.clone());
    }
    files.sort();

    if files.is_empty() {
        eprintln!(
            "no files matched under '{}' (extensions: {})",
            args.root.display(),
            if args.exts.is_empty() {
                "any".to_string()
            } else {
                args.exts.join(", ")
            }
        );
        return ExitCode::FAILURE;
    }

    let rules = builtin_rules();
    let mut hits: BTreeMap<String, usize> = BTreeMap::new();
    let mut samples: BTreeMap<String, Vec<Sample>> = BTreeMap::new();
    let mut total_lines = 0usize;
    let mut total_bytes = 0usize;
    let mut skipped = 0usize;

    let started = Instant::now();
    for path in &files {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("  skipping '{}': {e}", path.display());
                skipped += 1;
                continue;
            }
        };
        total_bytes += bytes.len();
        // Lossy rather than read_to_string: real logs contain invalid
        // UTF-8 often enough that failing the whole file would silently
        // shrink the corpus.
        let text = String::from_utf8_lossy(&bytes);
        let starts = line_starts(&text);
        // starts.len() counts one extra for a file ending in a newline
        // (the offset just past it). `starts` itself must keep that entry
        // so line_text can bound the final line, so adjust only the count
        // — this feeds the per-1k-lines metric, which is the number the
        // whole report turns on.
        total_lines += if text.is_empty() {
            0
        } else if text.ends_with('\n') {
            starts.len() - 1
        } else {
            starts.len()
        };

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        for finding in detect(&text, &rules) {
            *hits.entry(finding.rule_id.clone()).or_insert(0) += 1;
            let bucket = samples.entry(finding.rule_id.clone()).or_default();
            if bucket.len() < args.samples {
                let line = line_of(&starts, finding.start);
                bucket.push(Sample {
                    file: name.clone(),
                    line,
                    matched: truncate(&finding.matched, 60),
                    context: truncate(line_text(&text, &starts, line), 100),
                });
            }
        }
    }
    let elapsed = started.elapsed();

    let total_hits: usize = hits.values().sum();
    let per_k = |n: usize| {
        if total_lines == 0 {
            0.0
        } else {
            n as f64 * 1000.0 / total_lines as f64
        }
    };

    println!(
        "\nScanned {} file(s), {} lines, {} bytes in {:.2}s{}",
        commas(files.len()),
        commas(total_lines),
        commas(total_bytes),
        elapsed.as_secs_f64(),
        if skipped > 0 {
            format!(" ({skipped} skipped)")
        } else {
            String::new()
        }
    );
    println!("{} of {} rules fired\n", hits.len(), rules.len());

    // Sorted by hit count descending, then by id, so two runs of the same
    // corpus produce byte-identical output and can be diffed.
    let mut ranked: Vec<(&String, &usize)> = hits.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

    println!("{:<26}{:>10}{:>16}", "rule", "hits", "per 1k lines");
    println!("{}", "-".repeat(52));
    for (id, n) in &ranked {
        println!("{:<26}{:>10}{:>16.1}", id, commas(**n), per_k(**n));
    }
    println!("{}", "-".repeat(52));
    println!(
        "{:<26}{:>10}{:>16.1}",
        "TOTAL",
        commas(total_hits),
        per_k(total_hits)
    );

    if args.samples > 0 {
        println!("\n--- samples ---");
        for (id, n) in &ranked {
            println!("\n{id}  ({} hits)", commas(**n));
            for s in samples.get(*id).map(|v| v.as_slice()).unwrap_or(&[]) {
                println!("  {}:{}  matched {:?}", s.file, s.line, s.matched);
                println!("      {}", s.context);
            }
        }
    }

    println!();
    ExitCode::SUCCESS
}
