use redactify_core::{builtin_rules, detect, Finding};

/// Scan raw text with the builtin rules. First proof of the IPC
/// boundary: all detection stays in Rust; the frontend only ever
/// receives findings.
#[tauri::command]
fn scan_text(text: String) -> Vec<Finding> {
    detect(&text, &builtin_rules())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![scan_text])
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
