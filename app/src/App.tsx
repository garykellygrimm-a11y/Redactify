import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { TopBar } from "./components/TopBar";
import { Sidebar } from "./components/Sidebar";
import { DocumentView } from "./components/DocumentView";
import { VerdictStrip } from "./components/VerdictStrip";
import { ExportSuccess } from "./components/ExportSuccess";
import { toggleTheme } from "./theme";
import {
  Review,
  decide,
  decideRule,
  emptyReview,
  focusNext,
  focusPrev,
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
  const [error, setError] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const dragging = useRef(false);

  const loadPath = useCallback(async (path: string) => {
    try {
      setError(null);
      setExportResult(null);
      const result = await invoke<ScanOutcome>("open_file", { path });
      setOutcome(result);
      setReview(emptyReview(result.findings.length));
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const browse = useCallback(async () => {
    const picked = await openDialog({ multiple: false, directory: false });
    if (typeof picked === "string") await loadPath(picked);
  }, [loadPath]);

  const loadRules = useCallback(async () => {
    const picked = await openDialog({
      multiple: false,
      directory: false,
      filters: [{ name: "Rules (TOML)", extensions: ["toml"] }],
    });
    if (typeof picked !== "string") return;
    try {
      setError(null);
      const result = await invoke<RulesOutcome>("load_rules", { path: picked });
      setRulesInfo({ path: result.rules_path, count: result.rule_count });
      if (result.rescanned) {
        setOutcome(result.rescanned);
        setReview(emptyReview(result.rescanned.findings.length));
        setExportResult(null);
      }
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const closeDocument = useCallback(async () => {
    await invoke("close_document");
    setOutcome(null);
    setReview(emptyReview(0));
    setExportResult(null);
    setError(null);
  }, []);

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
        case "toggle_theme":
          toggleTheme();
          break;
      }
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [browse, loadRules, closeDocument]);

  // Keyboard doctrine: j/k walk, a/r decide, A/R decide-by-rule, u undo.
  useEffect(() => {
    if (!outcome || exportResult) return;
    function onKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement) return;
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
  }, [outcome, review.focused, exportResult]);

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
      <TopBar rulesInfo={rulesInfo} />
      {error && (
        <div className="border-b border-border bg-pending-soft px-4 py-2 text-sm text-pending">
          {error}
        </div>
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
            outcome={outcome}
            review={review}
            dragOver={dragOver}
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
    </div>
  );
}

export default App;
