import type { ScanOutcome } from "../App";

interface Props {
  outcome: ScanOutcome | null;
}

export function Sidebar({ outcome }: Props) {
  if (!outcome) {
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

  const counts = new Map<string, number>();
  for (const f of outcome.findings) {
    counts.set(f.rule_id, (counts.get(f.rule_id) ?? 0) + 1);
  }
  const groups = [...counts.entries()].sort((a, b) =>
    a[0].localeCompare(b[0]),
  );

  return (
    <aside className="flex h-full flex-col overflow-y-auto bg-surface-raised">
      <div className="border-b border-border px-4 py-3">
        <div className="text-sm font-medium">
          {outcome.findings.length} finding
          {outcome.findings.length === 1 ? "" : "s"}
        </div>
        <div className="mt-0.5 text-xs text-muted">
          {outcome.line_count.toLocaleString()} lines ·{" "}
          {outcome.elapsed_ms} ms
        </div>
      </div>
      <ul className="p-2">
        {groups.map(([ruleId, n]) => (
          <li
            key={ruleId}
            className="flex items-center justify-between rounded-md px-2 py-1.5 text-sm hover:bg-surface-sunken"
          >
            <span className="font-mono text-[13px]">{ruleId}</span>
            <span className="text-muted">{n}</span>
          </li>
        ))}
      </ul>
    </aside>
  );
}
