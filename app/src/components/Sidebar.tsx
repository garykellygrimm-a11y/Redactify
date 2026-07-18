import { useState } from "react";
import type { ScanOutcome } from "../App";
import { ruleHue } from "./DocumentView";

interface Props {
  outcome: ScanOutcome | null;
  focusedFinding: number | null;
  onFocus: (index: number) => void;
}

export function Sidebar({ outcome, focusedFinding, onFocus }: Props) {
  const [open, setOpen] = useState<Record<string, boolean>>({});

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

  const ruleIds = [...new Set(outcome.findings.map((f) => f.rule_id))].sort();
  const groups = ruleIds.map((id) => ({
    id,
    members: outcome.findings
      .map((f, i) => ({ f, i }))
      .filter(({ f }) => f.rule_id === id),
  }));

  return (
    <aside className="flex h-full flex-col overflow-y-auto bg-surface-raised">
      <div className="border-b border-border px-4 py-3">
        <div className="text-sm font-medium">
          {outcome.findings.length} finding
          {outcome.findings.length === 1 ? "" : "s"}
        </div>
        <div className="mt-0.5 text-xs text-muted">
          {outcome.line_count.toLocaleString()} lines · {outcome.elapsed_ms} ms
        </div>
      </div>
      <div className="p-2">
        {groups.map(({ id, members }) => (
          <div key={id} className="mb-1">
            <button
              onClick={() => setOpen((o) => ({ ...o, [id]: !o[id] }))}
              className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-surface-sunken"
            >
              <span
                className="h-2.5 w-2.5 shrink-0 rounded-full"
                style={{
                  background: `var(--rule-${ruleHue(id, ruleIds)})`,
                }}
              />
              <span className="font-mono text-[13px]">{id}</span>
              <span className="ml-auto text-muted">{members.length}</span>
            </button>
            {open[id] && (
              <ul className="mt-0.5 border-l border-border pl-3 ml-3">
                {members.map(({ f, i }) => (
                  <li key={i}>
                    <button
                      onClick={() => onFocus(i)}
                      className={`w-full truncate rounded px-2 py-1 text-left font-mono text-xs hover:bg-surface-sunken ${
                        i === focusedFinding
                          ? "bg-accent-soft text-accent"
                          : "text-muted"
                      }`}
                      title={f.matched}
                    >
                      {f.matched}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        ))}
      </div>
    </aside>
  );
}
