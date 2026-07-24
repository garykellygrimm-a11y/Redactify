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

  // Global: "?" opens/closes the shortcuts panel regardless of whether a
  // document is loaded. Deliberately separate from the review-key effect
  // below, which is gated on having an open document — there's nothing
  // document-specific about wanting to see the shortcut list.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement) return;
      if (e.key === "?") {
        setHelpOpen((h) => !h);
        e.preventDefault();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
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

  // Current-match row tint is now a plain prop on DocumentView (see
  // currentSearchLine below) — cheap to re-render because virtualization
  // means only the visible rows are ever mounted. Scrolling the match
  // into view is the one thing that still has to happen imperatively,
  // since the target row may not be mounted yet; scrollToLine drives the
  // virtualizer directly instead of querying the DOM for it.
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
        const name = path.replace(/^.*[\\/]/, "");
        void getCurrentWindow().setTitle(`${name} — Redactify`);
      } catch (e) {
        setError(String(e));
      }
    },
    [confirmDiscard],
  );

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
      if (result.rescanned) {
        setOutcome(result.rescanned);
        setReview(emptyReview(result.rescanned.findings.length));
        setExportResult(null);
        setViewMode("before");
      }
    } catch (e) {
      setError(String(e));
    }
  }, [outcome, confirmDiscard]);

  const closeDocument = useCallback(async () => {
    if (!(await confirmDiscard())) return;
    await invoke("close_document");
    setOutcome(null);
    setReview(emptyReview(0));
    setExportResult(null);
    setError(null);
    setViewMode("before");
    closeSearch();
    void getCurrentWindow().setTitle("Redactify");
  }, [closeSearch, confirmDiscard]);

  const doExport = useCallback(async () => {
    if (!outcome) return;
    const target = await saveDialog({
      defaultPath: defaultExportName(outcome.path),
    });
    if (typeof target !== "string") return; // user cancelled
    try {
      setError(null);
      const accepted = review.states
        .map((s, i) => (s === "accepted" ? i : -1))
        .filter((i) => i >= 0);
      setExportResult(
        await invoke<ExportOutcome>("export", {
          outputPath: target,
          accepted,
        }),
      );
    } catch (e) {
      setError(String(e));
    }
  }, [outcome, review.states]);

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
  }, [browse, loadRules, closeDocument]);

  // Keyboard doctrine: j/k walk, a/r decide, A/R decide-by-rule, u undo,
  // Ctrl+F search, Ctrl+D preview. Review keys stay quiet in inputs.
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
      if (e.target instanceof HTMLInputElement) return;
      if (e.key === "Escape") {
        closeSearch();
        return;
      }
      const focused = review.focused;
      switch (e.key) {
        case "j":
        case "ArrowDown":
          setReview((r) => focusNext(r));
          e.preventDefault();
          break;
        case "k":
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
        case "u":
          setReview((r) => undo(r));
          break;
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [outcome, review.focused, exportResult, closeSearch]);

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
