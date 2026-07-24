import { useEffect } from "react";

interface Props {
  open: boolean;
  onClose: () => void;
}

interface ShortcutGroup {
  title: string;
  items: { keys: string; description: string }[];
}

const GROUPS: ShortcutGroup[] = [
  {
    title: "Review",
    items: [
      { keys: "j / k", description: "Walk findings" },
      { keys: "a / r", description: "Accept / reject the focused finding" },
      {
        keys: "A / R",
        description: "Accept / reject every pending finding of the rule",
      },
      { keys: "u", description: "Undo the last decision" },
    ],
  },
  {
    title: "Document",
    items: [
      { keys: "Ctrl+F", description: "Search the document" },
      { keys: "Ctrl+D", description: "Toggle before/after output preview" },
    ],
  },
  {
    title: "File",
    items: [
      { keys: "Ctrl+O", description: "Open a file" },
      { keys: "Ctrl+L", description: "Load a custom rules file" },
      { keys: "Ctrl+W", description: "Close the current document" },
    ],
  },
  {
    title: "App",
    items: [
      { keys: "Ctrl+T", description: "Toggle light/dark theme" },
      { keys: "?", description: "Open or close this panel" },
      { keys: "Esc", description: "Close search, or this panel" },
    ],
  },
];

export function HelpPanel({ open, onClose }: Props) {
  // Self-contained: this panel manages its own Escape-to-close rather
  // than relying on App's review-key effect, which is gated on having a
  // document open — help should work with or without one.
  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-10 flex items-center justify-center bg-black/40"
      onClick={onClose}
    >
      <div
        className="w-[480px] max-w-[90vw] rounded-lg border border-border bg-surface-raised p-6 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-center justify-between">
          <div className="text-lg font-semibold">Keyboard shortcuts</div>
          <button
            onClick={onClose}
            className="rounded-md px-2 py-1 text-sm text-muted hover:bg-surface-sunken"
            title="Close (Esc)"
          >
            ✕
          </button>
        </div>
        <div className="space-y-4">
          {GROUPS.map((group) => (
            <div key={group.title}>
              <div className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted">
                {group.title}
              </div>
              <div className="space-y-1">
                {group.items.map((item) => (
                  <div
                    key={item.keys}
                    className="flex items-center justify-between gap-4 text-sm"
                  >
                    <span className="text-text">{item.description}</span>
                    <kbd className="shrink-0 rounded border border-border bg-surface-sunken px-1.5 py-0.5 font-mono text-xs text-muted">
                      {item.keys}
                    </kbd>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
