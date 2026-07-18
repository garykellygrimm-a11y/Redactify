use std::time::Instant;

use redactify_core::{builtin_rules, detect, Finding};
use serde::Serialize;

/// Everything the UI needs after opening a file: the text to display,
/// the findings to review, and honest stats for the scan moment.
#[derive(Serialize)]
struct ScanOutcome {
    path: String,
    text: String,
    findings: Vec<Finding>,
    line_count: usize,
    elapsed_ms: u128,
}

/// Read and scan a file. All I/O and detection stay in Rust; the
/// frontend only ever supplies a path and renders the outcome.
#[tauri::command]
fn open_file(path: String) -> Result<ScanOutcome, String> {
    let start = Instant::now();
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read '{path}': {e}"))?;
    let findings = detect(&text, &builtin_rules());
    Ok(ScanOutcome {
        line_count: text.lines().count(),
        elapsed_ms: start.elapsed().as_millis(),
        findings,
        text,
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
