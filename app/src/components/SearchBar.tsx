import { useEffect, useRef } from "react";

interface Props {
  query: string;
  matchCount: number;
  currentMatch: number; // 0-based; -1 when none
  onQuery: (q: string) => void;
  onNext: () => void;
  onPrev: () => void;
  onClose: () => void;
}

export function SearchBar({
  query,
  matchCount,
  currentMatch,
  onQuery,
  onNext,
  onPrev,
  onClose,
}: Props) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  return (
    <div className="flex items-center gap-2 border-b border-border bg-surface-raised px-4 py-1.5">
      <input
        ref={inputRef}
        value={query}
        onChange={(e) => onQuery(e.currentTarget.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.shiftKey ? onPrev : onNext)();
          if (e.key === "Escape") onClose();
        }}
        placeholder="Search document…"
        className="w-64 rounded-md border border-border bg-surface-sunken px-2 py-1 font-mono text-sm outline-none focus:border-accent focus:ring-2 focus:ring-accent"
      />
      <span className="text-xs text-muted">
        {query
          ? matchCount > 0
            ? `${currentMatch + 1} of ${matchCount}`
            : "No matches"
          : ""}
      </span>
      <button
        onClick={onPrev}
        disabled={matchCount === 0}
        className="rounded border border-border px-2 py-0.5 text-xs text-muted hover:bg-surface-sunken disabled:opacity-40"
        title="Previous (Shift+Enter)"
      >
        ↑
      </button>
      <button
        onClick={onNext}
        disabled={matchCount === 0}
        className="rounded border border-border px-2 py-0.5 text-xs text-muted hover:bg-surface-sunken disabled:opacity-40"
        title="Next (Enter)"
      >
        ↓
      </button>
      <button
        onClick={onClose}
        className="ml-auto rounded px-2 py-0.5 text-xs text-muted hover:bg-surface-sunken"
        title="Close (Esc)"
      >
        ✕
      </button>
    </div>
  );
}
