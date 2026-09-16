use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use redactify_core::{
    builtin_rules, compile_preview_rule, detect, glob_to_regex, load_rules_file, merge_rules,
    redact, sha256_hex, Disposition, Finding, Manifest, Rule, RuleInfo,
};
use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{Emitter, Manager, State};

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
struct OpenDocument {
    path: String,
    text: String,
    findings: Vec<Finding>,
}

/// Session state: the active rule set (builtins until a rules file is
/// loaded), the open document if any, and recently opened file paths
/// (most-recent-first, capped — see RECENT_FILES_CAP).
struct AppState {
    rules: Mutex<Vec<Rule>>,
    /// Tracked separately: `rules` holds the merged set, where a user rule
    /// replaces a builtin of the same id.
    user_rule_ids: Mutex<HashSet<String>>,
    document: Mutex<Option<OpenDocument>>,
    recent: Mutex<Vec<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            rules: Mutex::new(builtin_rules()),
            user_rule_ids: Mutex::new(HashSet::new()),
            document: Mutex::new(None),
            recent: Mutex::new(Vec::new()),
        }
    }
}

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

/// One rule as the UI sees it: the serializable core view, plus where it
/// came from. `source` is flattened alongside the RuleInfo fields, so the
/// frontend sees one flat object rather than a nested one.
#[derive(Serialize)]
struct RuleView {
    #[serde(flatten)]
    info: RuleInfo,
    /// "builtin" or "user". A user rule sharing a builtin's id has
    /// replaced it.
    source: &'static str,
}

#[derive(Serialize)]
struct PreviewLine {
    index: usize,
    segments: Vec<Segment>,
}

/// How to read the text the user typed.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum PatternSyntax {
    Regex,
    Glob,
}

#[derive(Serialize)]
struct PatternPreview {
    /// The regex actually run — differs from the input in glob mode.
    regex: String,
    match_count: usize,
    lines: Vec<PreviewLine>,
    /// `match_count` stays exact when this is true.
    truncated: bool,
}

/// Result of loading a rules file: what's now active, and — if a document
/// was open — the re-scanned outcome (the review session starts over).
#[derive(Serialize)]
struct RulesOutcome {
    rules_path: String,
    rule_count: usize,
    rescanned: Option<ScanOutcome>,
}

/// Split `text` into lines of alternating plain/finding segments.
fn segment(text: &str, findings: &[Finding]) -> Vec<Vec<Segment>> {
    let mut lines: Vec<Vec<Segment>> = Vec::new();
    let mut current: Vec<Segment> = Vec::new();
    let mut cursor = 0;
    let mut next = 0;

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

/// Scan `text` with `rules` and build the outcome + document record.
fn scan(path: String, text: String, rules: &[Rule]) -> (ScanOutcome, OpenDocument) {
    let start = Instant::now();
    let findings = detect(&text, rules);
    let lines = segment(&text, &findings);
    let outcome = ScanOutcome {
        line_count: lines.len(),
        elapsed_ms: start.elapsed().as_millis(),
        lines,
        findings: findings.clone(),
        path: path.clone(),
    };
    (
        outcome,
        OpenDocument {
            path,
            text,
            findings,
        },
    )
}

// ---------------------------------------------------------------------------
// Recent files: persisted as a small JSON array in the app's data dir.
// Deliberately hand-rolled rather than a plugin — it's a handful of lines
// and the project already depends on serde_json for manifests.
// ---------------------------------------------------------------------------

const RECENT_FILES_CAP: usize = 5;

fn recent_files_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("recent_files.json"))
}

/// Load the persisted recent-files list, dropping any entry whose file no
/// longer exists — a stale path in this menu (moved/deleted file) is worse
/// than no entry at all.
fn load_recent(app: &tauri::AppHandle) -> Vec<String> {
    let Some(path) = recent_files_path(app) else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let list: Vec<String> = serde_json::from_str(&text).unwrap_or_default();
    list.into_iter()
        .filter(|p| Path::new(p).exists())
        .take(RECENT_FILES_CAP)
        .collect()
}

fn save_recent(app: &tauri::AppHandle, recent: &[String]) {
    let Some(path) = recent_files_path(app) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(recent) {
        let _ = std::fs::write(&path, json);
    }
}

/// Move `path` to the front of `recent`, deduplicated, capped.
fn touch_recent(recent: &mut Vec<String>, path: &str) {
    recent.retain(|p| p != path);
    recent.insert(0, path.to_string());
    recent.truncate(RECENT_FILES_CAP);
}

/// Build the native menu, including a fresh "Open Recent" submenu.
/// Called at startup and again every time `recent` changes, since Tauri
/// menus are static once set — a changing recent-files list means
/// rebuilding and re-setting the whole menu, not editing one item in place.
fn build_menu(app: &tauri::AppHandle, recent: &[String]) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", "Open…", true, Some("CmdOrCtrl+O"))?;

    let recent_submenu = Submenu::new(app, "Open Recent", true)?;
    if recent.is_empty() {
        let none = MenuItem::with_id(app, "recent_none", "No Recent Files", false, None::<&str>)?;
        recent_submenu.append(&none)?;
    } else {
        for (i, path) in recent.iter().enumerate() {
            let label = Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());
            let item = MenuItem::with_id(app, format!("recent:{i}"), label, true, None::<&str>)?;
            recent_submenu.append(&item)?;
        }
        recent_submenu.append(&PredefinedMenuItem::separator(app)?)?;
        let clear = MenuItem::with_id(
            app,
            "clear_recent",
            "Clear Recent Files",
            true,
            None::<&str>,
        )?;
        recent_submenu.append(&clear)?;
    }

    let rules = MenuItem::with_id(app, "load_rules", "Load Rules…", true, Some("CmdOrCtrl+L"))?;
    let save = MenuItem::with_id(app, "save", "Save", true, Some("CmdOrCtrl+S"))?;
    let export = MenuItem::with_id(app, "export", "Export…", true, Some("CmdOrCtrl+E"))?;
    let close_doc = MenuItem::with_id(
        app,
        "close_document",
        "Close Document",
        true,
        Some("CmdOrCtrl+W"),
    )?;
    let file = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &open,
            &recent_submenu,
            &rules,
            &PredefinedMenuItem::separator(app)?,
            &save,
            &export,
            &PredefinedMenuItem::separator(app)?,
            &close_doc,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some("Exit"))?,
        ],
    )?;

    let preview = MenuItem::with_id(
        app,
        "toggle_preview",
        "Before / After Preview",
        true,
        Some("CmdOrCtrl+D"),
    )?;
    let theme = MenuItem::with_id(
        app,
        "toggle_theme",
        "Toggle Theme",
        true,
        Some("CmdOrCtrl+T"),
    )?;
    let view = Submenu::with_items(app, "View", true, &[&preview, &theme])?;

    Menu::with_items(app, &[&file, &view])
}

/// Read and scan a file with the active rule set. Also registers the path
/// in the recent-files list and rebuilds the menu to reflect it — every
/// way a file gets opened (Browse, drag-drop, or a recent-file click) goes
/// through this one command, so this is the single place that needs to know.
#[tauri::command]
fn open_file(
    path: String,
    state: State<AppState>,
    app: tauri::AppHandle,
) -> Result<ScanOutcome, String> {
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("Could not read '{path}': {e}"))?;
    let rules = state.rules.lock().unwrap();
    let (outcome, doc) = scan(path.clone(), text, &rules);
    *state.document.lock().unwrap() = Some(doc);

    let recent_snapshot = {
        let mut recent = state.recent.lock().unwrap();
        touch_recent(&mut recent, &path);
        recent.clone()
    };
    save_recent(&app, &recent_snapshot);
    if let Ok(menu) = build_menu(&app, &recent_snapshot) {
        let _ = app.set_menu(menu);
    }

    Ok(outcome)
}

/// Load a TOML rules file: fail-fast validation, merge over builtins,
/// become the session's active rule set. If a document is open, re-scan
/// it now — new rules silently NOT applying to the visible document
/// would be a false sense of coverage.
#[tauri::command]
fn load_rules(path: String, state: State<AppState>) -> Result<RulesOutcome, String> {
    let user = load_rules_file(&PathBuf::from(&path)).map_err(|e| e.to_string())?;
    // Capture which ids the user supplied BEFORE merging — afterwards the
    // two are indistinguishable.
    let user_ids: HashSet<String> = user.iter().map(|r| r.id.clone()).collect();
    let merged = merge_rules(builtin_rules(), user);
    let rule_count = merged.len();
    *state.user_rule_ids.lock().unwrap() = user_ids;

    let mut rules = state.rules.lock().unwrap();
    *rules = merged;

    let mut doc_guard = state.document.lock().unwrap();
    let rescanned = doc_guard.take().map(|doc| {
        let (outcome, new_doc) = scan(doc.path, doc.text, &rules);
        *doc_guard = Some(new_doc);
        outcome
    });

    Ok(RulesOutcome {
        rules_path: path,
        rule_count,
        rescanned,
    })
}

/// Every rule in the active set, for the rules panel. Independent of the
/// open document: you can inspect and (later) edit rules with nothing
/// loaded.
#[tauri::command]
fn list_rules(state: State<AppState>) -> Vec<RuleView> {
    let rules = state.rules.lock().unwrap();
    let user_ids = state.user_rule_ids.lock().unwrap();
    rules
        .iter()
        .map(|rule| RuleView {
            source: if user_ids.contains(&rule.id) {
                "user"
            } else {
                "builtin"
            },
            info: rule.info(),
        })
        .collect()
}

const PREVIEW_LINE_CAP: usize = 200;

/// Runs a candidate pattern against the open document without touching the
/// session rule set. Size-capped, since it compiles whatever is typed.
#[tauri::command]
fn preview_pattern(
    pattern: String,
    syntax: PatternSyntax,
    state: State<AppState>,
) -> Result<PatternPreview, String> {
    let regex = match syntax {
        PatternSyntax::Glob => glob_to_regex(&pattern),
        PatternSyntax::Regex => pattern,
    };

    let empty = |regex: String| PatternPreview {
        regex,
        match_count: 0,
        lines: Vec::new(),
        truncated: false,
    };

    if regex.is_empty() {
        return Ok(empty(regex));
    }

    let rule = compile_preview_rule(&regex).map_err(|e| e.to_string())?;

    let guard = state.document.lock().unwrap();
    let Some(doc) = guard.as_ref() else {
        return Ok(empty(regex));
    };

    let findings = detect(&doc.text, std::slice::from_ref(&rule));
    let match_count = findings.len();

    let mut lines = Vec::new();
    let mut truncated = false;
    for (index, segments) in segment(&doc.text, &findings).into_iter().enumerate() {
        if !segments.iter().any(|s| s.finding.is_some()) {
            continue;
        }
        if lines.len() >= PREVIEW_LINE_CAP {
            truncated = true;
            break;
        }
        lines.push(PreviewLine { index, segments });
    }

    Ok(PatternPreview {
        regex,
        match_count,
        lines,
        truncated,
    })
}

/// Drop the held document — the "start over" verb behind File > Close.
/// The active rule set survives; it is session state, not document state.
#[tauri::command]
fn close_document(state: State<AppState>) {
    *state.document.lock().unwrap() = None;
}

/// Apply the reviewer's verdicts: write the redacted file and manifest.
#[tauri::command]
fn export(
    output_path: String,
    accepted: Vec<usize>,
    state: State<AppState>,
) -> Result<ExportOutcome, String> {
    let guard = state.document.lock().unwrap();
    let doc = guard.as_ref().ok_or("No document is open")?;
    let rules = state.rules.lock().unwrap();

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
        &rules,
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
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            open_file,
            load_rules,
            list_rules,
            preview_pattern,
            close_document,
            export
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let recent = load_recent(&handle);
            *app.state::<AppState>().recent.lock().unwrap() = recent.clone();

            let menu = build_menu(&handle, &recent)?;
            app.set_menu(menu)?;

            app.on_menu_event(|app, event| {
                let id = event.id().0.clone();

                // Recent-file clicks carry a path, so they go out on their
                // own event rather than the generic "menu" channel, which
                // only ever carried bare action ids.
                if let Some(idx_str) = id.strip_prefix("recent:") {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        let path = app
                            .state::<AppState>()
                            .recent
                            .lock()
                            .unwrap()
                            .get(idx)
                            .cloned();
                        if let Some(path) = path {
                            let _ = app.emit("open_path", path);
                        }
                    }
                    return;
                }

                if id == "clear_recent" {
                    app.state::<AppState>().recent.lock().unwrap().clear();
                    save_recent(app, &[]);
                    if let Ok(menu) = build_menu(app, &[]) {
                        let _ = app.set_menu(menu);
                    }
                    return;
                }

                let _ = app.emit("menu", id);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
