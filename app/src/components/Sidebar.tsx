export function Sidebar() {
  return (
    <aside className="flex h-full flex-col overflow-y-auto bg-surface-raised">
      <div className="border-b border-border px-4 py-3 text-sm font-medium">
        Findings
      </div>
      <div className="flex flex-1 items-center justify-center p-6 text-center text-sm text-muted">
        No file loaded yet.
        <br />
        Findings will appear here, grouped by rule.
      </div>
    </aside>
  );
}
