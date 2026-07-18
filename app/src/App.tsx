import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { TopBar } from "./components/TopBar";
import { Sidebar } from "./components/Sidebar";
import { DocumentView } from "./components/DocumentView";
import { VerdictStrip } from "./components/VerdictStrip";
import "./App.css";

export interface Finding {
  start: number;
  end: number;
  rule_id: string;
  matched: string;
}

export interface ScanOutcome {
  path: string;
  text: string;
  findings: Finding[];
  line_count: number;
  elapsed_ms: number;
}

const SIDEBAR_MIN = 220;
const SIDEBAR_MAX = 480;

function App() {
  const [sidebarWidth, setSidebarWidth] = useState(300);
  const [outcome, setOutcome] = useState<ScanOutcome | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const dragging = useRef(false);

  const loadPath = useCallback(async (path: string) => {
    try {
      setError(null);
      setOutcome(await invoke<ScanOutcome>("open_file", { path }));
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const browse = useCallback(async () => {
    const picked = await openDialog({ multiple: false, directory: false });
    if (typeof picked === "string") await loadPath(picked);
  }, [loadPath]);

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

  return (
    <div className="flex h-screen flex-col font-sans">
      <TopBar />
      {error && (
        <div className="border-b border-border bg-pending-soft px-4 py-2 text-sm text-pending">
          {error}
        </div>
      )}
      <div className="flex min-h-0 flex-1">
        <div style={{ width: sidebarWidth }} className="shrink-0">
          <Sidebar outcome={outcome} />
        </div>
        <div
          onMouseDown={onDividerDown}
          className="w-1 shrink-0 cursor-col-resize bg-border hover:bg-accent"
          title="Drag to resize"
        />
        <div className="min-w-0 flex-1">
          <DocumentView outcome={outcome} dragOver={dragOver} onBrowse={browse} />
        </div>
      </div>
      <VerdictStrip outcome={outcome} />
    </div>
  );
}

export default App;
