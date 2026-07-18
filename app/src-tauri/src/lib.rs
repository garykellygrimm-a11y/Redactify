use std::sync::Mutex;
use std::time::Instant;

use redactify_core::{builtin_rules, detect, redact, sha256_hex, Disposition, Finding, Manifest};
use serde::Serialize;
use tauri::State;

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

/// The open document, held on the Rust side between open and export.
/// The frontend reviews segments; Rust keeps the text and findings that
/// export operates on — the UI never round-trips document content.
struct OpenDocument {
    text: String,
    findings: Vec<Finding>,
}

#[derive(Default)]
struct AppState(Mutex<Option<OpenDocument>>);

/// What the success screen shows.
#[derive(Serialize)]
struct ExportOutcome {
    output_path: String,
    manifest_path: String,
    source_sha256: String,
    output_sha256: String,
    applied_count: usize,
    rejected_count: usize,
}

/// Split `text` into lines of alternating plain/finding segments.
/// Findings are sorted and non-overlapping (detect guarantees it), so a
/// single forward walk with one cursor suffices — same shape as redact().
fn segment(text: &str, findings: &[Finding]) -> Vec<Vec<Segment>> {
    let mut lines: Vec<Vec<Segment>> = Vec::new();
    let mut current: Vec<Segment> = Vec::new();
    let mut cursor = 0; // byte offset into text
    let mut next = 0; // index of the next finding to place

    for line in text.split_inclusive('\n') {
        let line_start = cursor;
        let line_end = cursor + line.len();
        let content_end = line_end
            - line
                .chars()
                .rev()
                .take_while(|c| *c == '\n' || *c == '\r')
                .count();

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
fn open_file(path: String, state: State<AppState>) -> Result<ScanOutcome, String> {
    let start = Instant::now();
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("Could not read '{path}': {e}"))?;
    let findings = detect(&text, &builtin_rules());
    let lines = segment(&text, &findings);

    let outcome = ScanOutcome {
        line_count: lines.len(),
        elapsed_ms: start.elapsed().as_millis(),
        lines,
        findings: findings.clone(),
        path,
    };
    *state.0.lock().unwrap() = Some(OpenDocument { text, findings });
    Ok(outcome)
}

/// Apply the reviewer's verdicts: write the redacted file and the
/// manifest alongside it. `accepted` holds indices into the findings
/// of the currently open document; everything else is Rejected.
#[tauri::command]
fn export(
    output_path: String,
    accepted: Vec<usize>,
    state: State<AppState>,
) -> Result<ExportOutcome, String> {
    let guard = state.0.lock().unwrap();
    let doc = guard.as_ref().ok_or("No document is open")?;

    let mut dispositions = vec![Disposition::Rejected; doc.findings.len()];
    for &i in &accepted {
        *dispositions
            .get_mut(i)
            .ok_or_else(|| format!("Invalid finding index {i}"))? = Disposition::Accepted;
    }

    let applied: Vec<Finding> = doc
        .findings
        .iter()
        .zip(&dispositions)
        .filter(|(_, d)| **d == Disposition::Accepted)
        .map(|(f, _)| f.clone())
        .collect();
    let redacted = redact(&doc.text, &applied);

    let tool = format!("redactify {}", env!("CARGO_PKG_VERSION"));
    let manifest = Manifest::new(
        &tool,
        &doc.text,
        &redacted,
        &builtin_rules(),
        &doc.findings,
        &dispositions,
    );

    let manifest_path = format!("{output_path}.manifest.json");
    std::fs::write(&output_path, &redacted)
        .map_err(|e| format!("Could not write '{output_path}': {e}"))?;
    std::fs::write(
        &manifest_path,
        manifest.to_json().map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Could not write '{manifest_path}': {e}"))?;

    Ok(ExportOutcome {
        source_sha256: sha256_hex(&doc.text),
        output_sha256: sha256_hex(&redacted),
        applied_count: applied.len(),
        rejected_count: doc.findings.len() - applied.len(),
        output_path,
        manifest_path,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![open_file, export])
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
