import { useCallback, useEffect, useRef, useState } from "react";
import { TopBar } from "./components/TopBar";
import { Sidebar } from "./components/Sidebar";
import { DocumentView } from "./components/DocumentView";
import { VerdictStrip } from "./components/VerdictStrip";
import "./App.css";

const SIDEBAR_MIN = 220;
const SIDEBAR_MAX = 480;

function App() {
  const [sidebarWidth, setSidebarWidth] = useState(300);
  const dragging = useRef(false);

  const onDividerDown = useCallback(() => {
    dragging.current = true;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }, []);

  useEffect(() => {
    function onMove(e: MouseEvent) {
      if (!dragging.current) return;
      setSidebarWidth(
        Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, e.clientX)),
      );
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
      <div className="flex min-h-0 flex-1">
        <div style={{ width: sidebarWidth }} className="shrink-0">
          <Sidebar />
        </div>
        <div
          onMouseDown={onDividerDown}
          className="w-1 shrink-0 cursor-col-resize bg-border hover:bg-accent"
          title="Drag to resize"
        />
        <div className="min-w-0 flex-1">
          <DocumentView />
        </div>
      </div>
      <VerdictStrip />
    </div>
  );
}

export default App;
