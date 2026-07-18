use std::time::Instant;

use redactify_core::{builtin_rules, detect, Finding};
use serde::Serialize;

/// One render-ready piece of a line: plain text, or a slice of a finding.
#[derive(Serialize)]
struct Segment {
    text: String,
    /// Index into ScanOutcome.findings, or None for plain text.
    finding: Option<usize>,
}

/// Everything the UI needs after opening a file. `lines` carries the
/// document pre-segmented in Rust — the frontend does NO offset math,
/// because findings are UTF-8 byte offsets and JS strings are UTF-16;
/// converting in JS invites silent misalignment on non-ASCII input.
#[derive(Serialize)]
struct ScanOutcome {
    path: String,
    findings: Vec<Finding>,
    lines: Vec<Vec<Segment>>,
    line_count: usize,
    elapsed_ms: u128,
}

/// Split `text` into lines of alternating plain/finding segments.
/// Findings are sorted and non-overlapping (detect guarantees it), so a
/// single forward walk with one cursor suffices — same shape as redact().
fn segment(text: &str, findings: &[Finding]) -> Vec<Vec<Segment>> {
    let mut lines: Vec<Vec<Segment>> = Vec::new();
    let mut current: Vec<Segment> = Vec::new();
    let mut cursor = 0; // byte offset into text
    let mut next = 0; // index of the next finding to place

    // Walk line by line so findings can't straddle our rendering units.
    for line in text.split_inclusive('\n') {
        let line_start = cursor;
        let line_end = cursor + line.len();
        let content_end = line_end - line.chars().rev().take_while(|c| *c == '\n' || *c == '\r').count();

        let mut pos = line_start;
        while next < findings.len() && findings[next].start < content_end {
            let f = &findings[next];
            if f.start > pos {
                current.push(Segment {
                    text: text[pos..f.start].to_string(),
                    finding: None,
                });
            }
            current.push(Segment {
                text: text[f.start..f.end].to_string(),
                finding: Some(next),
            });
            pos = f.end;
            next += 1;
        }
        if pos < content_end {
            current.push(Segment {
                text: text[pos..content_end].to_string(),
                finding: None,
            });
        }

        lines.push(std::mem::take(&mut current));
        cursor = line_end;
    }
    if !current.is_empty() || text.ends_with('\n') {
        lines.push(current);
    }
    lines
}

/// Read and scan a file. All I/O, detection, and offset math stay in Rust.
#[tauri::command]
fn open_file(path: String) -> Result<ScanOutcome, String> {
    let start = Instant::now();
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read '{path}': {e}"))?;
    let findings = detect(&text, &builtin_rules());
    let lines = segment(&text, &findings);
    Ok(ScanOutcome {
        line_count: lines.len(),
        elapsed_ms: start.elapsed().as_millis(),
        lines,
        findings,
        path,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![open_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
