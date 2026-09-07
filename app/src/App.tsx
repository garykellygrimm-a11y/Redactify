import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  confirm,
  open as openDialog,
  save as saveDialog,
} from "@tauri-apps/plugin-dialog";
import { TopBar } from "./components/TopBar";
import { Sidebar } from "./components/Sidebar";
import {
  DocumentView,
  type DocumentViewHandle,
  type ViewMode,
} from "./components/DocumentView";
import { VerdictStrip } from "./components/VerdictStrip";
import { ExportSuccess } from "./components/ExportSuccess";
import { SearchBar } from "./components/SearchBar";
import { HelpPanel } from "./components/HelpPanel";
import { toggleTheme } from "./theme";
import {
  applyTextSize,
  loadTextSize,
  saveTextSize,
  stepTextSize,
  type TextSize,
} from "./textSize";
import {
  Review,
  decide,
  decideRule,
  emptyReview,
  focusNext,
  focusPrev,
  tally,
  undo,
} from "./review";
import "./App.css";

export interface Finding {
  start: number;
  end: number;
  rule_id: string;
  matched: string;
}

export interface Segment {
  text: string;
  finding: number | null;
}

export interface ScanOutcome {
  path: string;
  findings: Finding[];
  lines: Segment[][];
  line_count: number;
  elapsed_ms: number;
}

export interface ExportOutcome {
  output_path: string;
  manifest_path: string;
  source_sha256: string;
  output_sha256: string;
  applied_count: number;
  rejected_count: number;
}

export interface RulesOutcome {
  rules_path: string;
  rule_count: number;
  rescanned: ScanOutcome | null;
}

/// Mirrors the Rust RuleView: RuleInfo's fields flattened, plus where the
/// rule came from. finding_group is Option<usize> in Rust, so it arrives
/// as null rather than undefined when unset.
export interface RuleView {
  id: string;
  name: string;
  pattern: string;
  validated: boolean;
  finding_group: number | null;
  source: "builtin" | "user";
}

export interface PreviewLine {
  index: number;
  segments: Segment[];
}

export interface PatternPreview {
  match_count: number;
  lines: PreviewLine[];
  truncated: boolean;
}

export interface RulesInfo {
  path: string;
  count: number;
}

const SIDEBAR_MIN = 220;
const SIDEBAR_MAX = 480;

/** Default save name: insert `.redacted` before the extension. */
function defaultExportName(path: string): string {
  const base = path.replace(/^.*[\\/]/, "");
  const dot = base.lastIndexOf(".");
  return dot > 0
    ? `${base.slice(0, dot)}.redacted${base.slice(dot)}`
    : `${base}.redacted`;
}

function App() {
  const [sidebarWidth, setSidebarWidth] = useState(300);
  const [outcome, setOutcome] = useState<ScanOutcome | null>(null);
  const [review, setReview] = useState<Review>(emptyReview(0));
  const [rulesInfo, setRulesInfo] = useState<RulesInfo | null>(null);
  const [rules, setRules] = useState<RuleView[]>([]);
  const [previewPattern, setPreviewPattern] = useState("");
  const [preview, setPreview] = useState<PatternPreview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [exportResult, setExportResult] = useState<ExportOutcome | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("before");
  const [error, setError] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [currentMatch, setCurrentMatch] = useState(-1);
  const [textSize, setTextSize] = useState<TextSize>(loadTextSize());
  const [helpOpen, setHelpOpen] = useState(false);
  // Export destination remembered for this review session only — reset
  // whenever a new document loads, rules force a rescan, or the document
  // closes. Not persisted; "the place I'm saving THIS review" shouldn't
  // survive to a different file.
  const [lastExportPath, setLastExportPath] = useState<string | null>(null);
  const dragging = useRef(false);

  // Shadow of `review` for callbacks that must read the CURRENT state
  // without depending on it (dependency churn would re-subscribe the
  // menu listener on every decision).
  const reviewRef = useRef(review);
  useEffect(() => {
    reviewRef.current = review;
  }, [review]);

  // Push the current level to the DOM/localStorage whenever it changes.
  // main.tsx handles the very first application (before React mounts);
  // this effect takes over for changes made via the TopBar stepper.
  useEffect(() => {
    applyTextSize(textSize);
    saveTextSize(textSize);
  }, [textSize]);

  const changeTextSize = useCallback((direction: 1 | -1) => {
    setTextSize((cur) => stepTextSize(cur, direction));
  }, []);


  /** True if it's safe to discard the session; asks the user when not. */
  const confirmDiscard = useCallback(async (): Promise<boolean> => {
    const r = reviewRef.current;
    if (r.states.length === 0) return true;
    const { pending } = tally(r);
    const decided = r.log.length > 0;
    if (pending === 0 && !decided) return true; // nothing at stake
    const what =
      pending > 0
        ? `${pending} finding${pending === 1 ? "" : "s"} still undecided`
        : "your review decisions";
    return confirm(`Discard ${what}? This cannot be undone.`, {
      title: "Redactify",
      kind: "warning",
    });
  }, []);

  // Search matching walks every line, so it runs against a DEBOUNCED
  // copy of the query — typing stays instant on large files; the scan
  // fires once per pause instead of once per keystroke.
  useEffect(() => {
    const t = setTimeout(() => setDebouncedQuery(query), 200);
    return () => clearTimeout(t);
  }, [query]);

  // Line indices containing the query, case-insensitive.
  const matches = useMemo(() => {
    if (!outcome || !searchOpen || debouncedQuery.trim() === "") return [];
    const q = debouncedQuery.toLowerCase();
    const hits: number[] = [];
    outcome.lines.forEach((segments, i) => {
      const line = segments.map((s) => s.text).join("");
      if (line.toLowerCase().includes(q)) hits.push(i);
    });
    return hits;
  }, [outcome, searchOpen, debouncedQuery]);

  useEffect(() => {
    setCurrentMatch(matches.length > 0 ? 0 : -1);
  }, [matches]);

  // Scrolling to a match must stay imperative — the target row may not be
  // mounted.
  const documentViewRef = useRef<DocumentViewHandle>(null);
  useEffect(() => {
    if (currentMatch < 0) return;
    documentViewRef.current?.scrollToLine(matches[currentMatch]);
  }, [currentMatch, matches]);

  const nextMatch = useCallback(() => {
    if (matches.length > 0) setCurrentMatch((c) => (c + 1) % matches.length);
  }, [matches.length]);

  const prevMatch = useCallback(() => {
    if (matches.length > 0)
      setCurrentMatch((c) => (c - 1 + matches.length) % matches.length);
  }, [matches.length]);

  const closeSearch = useCallback(() => {
    setSearchOpen(false);
    setQuery("");
  }, []);

  const loadPath = useCallback(
    async (path: string) => {
      if (!(await confirmDiscard())) return;
      try {
        setError(null);
        setExportResult(null);
        setViewMode("before");
        const result = await invoke<ScanOutcome>("open_file", { path });
        setOutcome(result);
        setReview(emptyReview(result.findings.length));
        setLastExportPath(null);
        const name = path.replace(/^.*[\\/]/, "");
        void getCurrentWindow().setTitle(`${name} — Redactify`);
      } catch (e) {
        setError(String(e));
      }
    },
    [confirmDiscard],
  );

  const refreshRules = useCallback(async () => {
    try {
      setRules(await invoke<RuleView[]>("list_rules"));
    } catch (e) {
      // Non-fatal: the rules panel just stays empty. Scanning and review
      // don't depend on the frontend knowing the rule list.
      console.error("could not list rules:", e);
    }
  }, []);

  // The active rule set exists before any document does, so this loads on
  // mount rather than waiting for a file.
  useEffect(() => {
    void refreshRules();
  }, [refreshRules]);

  // Debounced — each call rescans the whole document.
  useEffect(() => {
    if (previewPattern === "") {
      setPreview(null);
      setPreviewError(null);
      return;
    }
    const timer = setTimeout(() => {
      invoke<PatternPreview>("preview_pattern", { pattern: previewPattern })
        .then((result) => {
          setPreview(result);
          setPreviewError(null);
        })
        .catch((e) => {
          setPreview(null);
          setPreviewError(String(e));
        });
    }, 250);
    return () => clearTimeout(timer);
  }, [previewPattern, outcome]);

  const browse = useCallback(async () => {
    const picked = await openDialog({ multiple: false, directory: false });
    if (typeof picked === "string") await loadPath(picked);
  }, [loadPath]);

  // Recent-file menu clicks carry a real path, unlike the generic "menu"
  // channel's bare action ids — Rust emits this separately so it can send
  // that payload.
  useEffect(() => {
    const unlisten = listen<string>("open_path", (event) => {
      void loadPath(event.payload);
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [loadPath]);

  const loadRules = useCallback(async () => {
    const picked = await openDialog({
      multiple: false,
      directory: false,
      filters: [{ name: "Rules (TOML)", extensions: ["toml"] }],
    });
    if (typeof picked !== "string") return;
    // A load with a document open re-scans and resets the review.
    if (outcome && !(await confirmDiscard())) return;
    try {
      setError(null);
      const result = await invoke<RulesOutcome>("load_rules", { path: picked });
      setRulesInfo({ path: result.rules_path, count: result.rule_count });
      void refreshRules();
      if (result.rescanned) {
        setOutcome(result.rescanned);
        setReview(emptyReview(result.rescanned.findings.length));
        setExportResult(null);
        setViewMode("before");
        setLastExportPath(null);
      }
    } catch (e) {
      setError(String(e));
    }
  }, [outcome, confirmDiscard, refreshRules]);

  // App-level shortcuts, ungated: Ctrl+O is most useful with nothing open.
  // Duplicated from the native menu accelerators, which never fire.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement) return;
      if (e.key === "?") {
        setHelpOpen((h) => !h);
        e.preventDefault();
        return;
      }
      if (!(e.ctrlKey || e.metaKey)) return;
      switch (e.key) {
        case "o":
          e.preventDefault();
          void browse();
          break;
        case "l":
          e.preventDefault();
          void loadRules();
          break;
        case "t":
          e.preventDefault();
          toggleTheme();
          break;
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [browse, loadRules]);

  const closeDocument = useCallback(async () => {
    if (!(await confirmDiscard())) return;
    await invoke("close_document");
    setOutcome(null);
    setReview(emptyReview(0));
    setExportResult(null);
    setError(null);
    setViewMode("before");
    setLastExportPath(null);
    closeSearch();
    void getCurrentWindow().setTitle("Redactify");
  }, [closeSearch, confirmDiscard]);

  // The pending check lives here too: a keyboard shortcut has no disabled
  // state to respect.
  const doExport = useCallback(async () => {
    if (!outcome) return;
    const { pending } = tally(review);
    if (pending > 0) {
      setError("Every finding must be decided before exporting.");
      return;
    }
    const target = await saveDialog({
      defaultPath: defaultExportName(outcome.path),
    });
    if (typeof target !== "string") return; // user cancelled
    try {
      setError(null);
      const accepted = review.states
        .map((s, i) => (s === "accepted" ? i : -1))
        .filter((i) => i >= 0);
      const result = await invoke<ExportOutcome>("export", {
        outputPath: target,
        accepted,
      });
      setLastExportPath(target);
      setExportResult(result);
    } catch (e) {
      setError(String(e));
    }
  }, [outcome, review]);

  // Save: reuse the last export destination this session, no dialog. The
  // very first save (or the first after a rescan/new document, which
  // clears lastExportPath) has nothing to reuse yet, so it falls back to
  // the same prompt-and-remember behavior as Export.
  const doSave = useCallback(async () => {
    if (!outcome) return;
    if (!lastExportPath) {
      await doExport();
      return;
    }
    const { pending } = tally(review);
    if (pending > 0) {
      setError("Every finding must be decided before saving.");
      return;
    }
    try {
      setError(null);
      const accepted = review.states
        .map((s, i) => (s === "accepted" ? i : -1))
        .filter((i) => i >= 0);
      const result = await invoke<ExportOutcome>("export", {
        outputPath: lastExportPath,
        accepted,
      });
      setExportResult(result);
    } catch (e) {
      setError(String(e));
    }
  }, [outcome, review, lastExportPath, doExport]);

  // Native menu events from Rust: one channel, routed by id.
  useEffect(() => {
    const unlisten = listen<string>("menu", (event) => {
      switch (event.payload) {
        case "open":
          void browse();
          break;
        case "load_rules":
          void loadRules();
          break;
        case "close_document":
          void closeDocument();
          break;
        case "save":
          void doSave();
          break;
        case "export":
          void doExport();
          break;
        case "toggle_preview":
          setViewMode((m) => (m === "before" ? "after" : "before"));
          break;
        case "toggle_theme":
          toggleTheme();
          break;
      }
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [browse, loadRules, closeDocument, doSave, doExport]);

  // Keyboard doctrine: arrows walk, a/r decide, Shift+A/R decide-by-rule,
  // Ctrl+Z undo, Ctrl+F search, Ctrl+D preview, Ctrl+S save, Ctrl+E
  // export, Ctrl+W close. Review keys stay quiet in inputs.
  useEffect(() => {
    if (!outcome || exportResult) return;
    function onKey(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "f") {
        setSearchOpen(true);
        e.preventDefault();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "d") {
        setViewMode((m) => (m === "before" ? "after" : "before"));
        e.preventDefault();
        return;
      }
      // Gated rather than app-level: closing a document only means
      // something when one is open.
      if ((e.ctrlKey || e.metaKey) && e.key === "w") {
        e.preventDefault();
        void closeDocument();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        void doSave();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "e") {
        e.preventDefault();
        void doExport();
        return;
      }
      if (e.target instanceof HTMLInputElement) return;
      if (e.key === "Escape") {
        closeSearch();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "z") {
        setReview((r) => undo(r));
        e.preventDefault();
        return;
      }
      const focused = review.focused;
      switch (e.key) {
        case "ArrowDown":
          setReview((r) => focusNext(r));
          e.preventDefault();
          break;
        case "ArrowUp":
          setReview((r) => focusPrev(r));
          e.preventDefault();
          break;
        case "a":
          if (focused !== null)
            setReview((r) => focusNext(decide(r, focused, "accepted")));
          break;
        case "r":
          if (focused !== null)
            setReview((r) => focusNext(decide(r, focused, "rejected")));
          break;
        case "A":
        case "R": {
          if (focused === null || !outcome) break;
          const rule = outcome.findings[focused].rule_id;
          const members = outcome.findings
            .map((f, i) => ({ f, i }))
            .filter(({ f }) => f.rule_id === rule)
            .map(({ i }) => i);
          setReview((r) =>
            decideRule(r, members, e.key === "A" ? "accepted" : "rejected"),
          );
          break;
        }
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [outcome, review.focused, exportResult, closeSearch, doSave, doExport, closeDocument]);

  // Native drag-and-drop: Tauri surfaces real file paths, which the
  // browser's own drop events cannot do inside a webview.
  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") setDragOver(true);
      else if (event.payload.type === "drop") {
        setDragOver(false);
        const first = event.payload.paths[0];
        if (first) void loadPath(first);
      } else setDragOver(false);
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [loadPath]);

  const onDividerDown = useCallback(() => {
    dragging.current = true;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }, []);

  useEffect(() => {
    function onMove(e: MouseEvent) {
      if (!dragging.current) return;
      setSidebarWidth(Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, e.clientX)));
    }
    function onUp() {
      dragging.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    }
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, []);

  const setFocus = useCallback(
    (index: number) => setReview((r) => ({ ...r, focused: index })),
    [],
  );

  const decideOne = useCallback(
    (index: number, verdict: "accepted" | "rejected") =>
      setReview((r) => decide(r, index, verdict)),
    [],
  );

  const decideGroup = useCallback(
    (indices: number[], verdict: "accepted" | "rejected") =>
      setReview((r) => decideRule(r, indices, verdict)),
    [],
  );

  return (
    <div className="flex h-screen flex-col font-sans">
      <TopBar
        rulesInfo={rulesInfo}
        textSize={textSize}
        onChangeTextSize={changeTextSize}
        onOpenHelp={() => setHelpOpen(true)}
        hasDocument={!!outcome}
        viewMode={viewMode}
        onSetViewMode={setViewMode}
      />
      {error && (
        <div className="border-b border-border bg-pending-soft px-4 py-2 text-sm text-pending">
          {error}
        </div>
      )}
      {searchOpen && outcome && (
        <SearchBar
          query={query}
          matchCount={matches.length}
          currentMatch={currentMatch}
          onQuery={setQuery}
          onNext={nextMatch}
          onPrev={prevMatch}
          onClose={closeSearch}
        />
      )}
      <div className="flex min-h-0 flex-1">
        <div style={{ width: sidebarWidth }} className="shrink-0">
          <Sidebar
            outcome={outcome}
            review={review}
            rules={rules}
            rulesPath={rulesInfo?.path ?? null}
            previewPattern={previewPattern}
            preview={preview}
            previewError={previewError}
            onPreviewPatternChange={setPreviewPattern}
            onFocus={setFocus}
            onDecide={decideOne}
            onDecideGroup={decideGroup}
          />
        </div>
        <div
          onMouseDown={onDividerDown}
          className="w-1 shrink-0 cursor-col-resize bg-border hover:bg-accent"
          title="Drag to resize"
        />
        <div className="min-w-0 flex-1">
          <DocumentView
            ref={documentViewRef}
            outcome={outcome}
            review={review}
            mode={viewMode}
            dragOver={dragOver}
            searchQuery={debouncedQuery}
            currentSearchLine={currentMatch >= 0 ? matches[currentMatch] : null}
            textSize={textSize}
            preview={preview}
            onBrowse={browse}
          />
        </div>
      </div>
      <VerdictStrip review={review} outcome={outcome} onExport={doExport} />
      {exportResult && (
        <ExportSuccess
          result={exportResult}
          onDismiss={() => setExportResult(null)}
        />
      )}
      <HelpPanel open={helpOpen} onClose={() => setHelpOpen(false)} />
    </div>
  );
}

export default App;
