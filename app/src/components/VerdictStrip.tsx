export function VerdictStrip() {
  return (
    <footer className="flex h-11 shrink-0 items-center gap-4 border-t border-border bg-surface-raised px-4 text-sm">
      <span className="text-muted">No findings yet</span>
      <button
        className="ml-auto rounded-md bg-accent px-4 py-1 font-medium text-surface-raised disabled:opacity-40"
        disabled
        title="Export unlocks when every finding is decided"
      >
        Export
      </button>
    </footer>
  );
}
