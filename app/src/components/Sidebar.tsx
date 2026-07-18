import { useState } from "react";
import type { ScanOutcome } from "../App";
import type { Review, Verdict } from "../review";
import { ruleHue } from "./DocumentView";

interface Props {
  outcome: ScanOutcome | null;
  review: Review;
  onFocus: (index: number) => void;
  onDecide: (index: number, verdict: Verdict) => void;
  onDecideGroup: (indices: number[], verdict: Verdict) => void;
}

const STATE_GLYPH = { pending: "·", accepted: "✓", rejected: "✕" } as const;

export function Sidebar({
  outcome,
  review,
  onFocus,
  onDecide,
  onDecideGroup,
}: Props) {
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
        {groups.map(({ id, members }) => {
          const indices = members.map(({ i }) => i);
          const pendingCount = indices.filter(
            (i) => review.states[i] === "pending",
          ).length;
          return (
            <div key={id} className="mb-1">
              <div className="group flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-surface-sunken">
                <button
                  onClick={() => setOpen((o) => ({ ...o, [id]: !o[id] }))}
                  className="flex min-w-0 flex-1 items-center gap-2 text-left"
                >
                  <span
                    className="h-2.5 w-2.5 shrink-0 rounded-full"
                    style={{ background: `var(--rule-${ruleHue(id, ruleIds)})` }}
                  />
                  <span className="truncate font-mono text-[13px]">{id}</span>
                  <span className="ml-auto text-muted">
                    {pendingCount > 0 ? `${pendingCount} pending` : "done"}
                  </span>
                </button>
                {pendingCount > 0 && (
                  <span className="hidden shrink-0 gap-1 group-hover:flex">
                    <button
                      onClick={() => onDecideGroup(indices, "accepted")}
                      className="rounded bg-accepted-soft px-1.5 text-xs text-accepted"
                      title={`Accept all pending ${id}`}
                    >
                      ✓ all
                    </button>
                    <button
                      onClick={() => onDecideGroup(indices, "rejected")}
                      className="rounded bg-surface-sunken px-1.5 text-xs text-rejected"
                      title={`Reject all pending ${id}`}
                    >
                      ✕ all
                    </button>
                  </span>
                )}
              </div>
              {open[id] && (
                <ul className="ml-3 mt-0.5 border-l border-border pl-3">
                  {members.map(({ f, i }) => {
                    const state = review.states[i];
                    return (
                      <li key={i} className="group/item flex items-center gap-1">
                        <button
                          onClick={() => onFocus(i)}
                          className={`min-w-0 flex-1 truncate rounded px-2 py-1 text-left font-mono text-xs hover:bg-surface-sunken ${
                            i === review.focused
                              ? "bg-accent-soft text-accent"
                              : state === "pending"
                                ? "text-text"
                                : "text-muted"
                          }`}
                          title={f.matched}
                        >
                          <span
                            className={
                              state === "accepted"
                                ? "text-accepted"
                                : state === "rejected"
                                  ? "text-rejected"
                                  : "text-pending"
                            }
                          >
                            {STATE_GLYPH[state]}
                          </span>{" "}
                          {f.matched}
                        </button>
                        <span className="hidden shrink-0 gap-1 group-hover/item:flex">
                          <button
                            onClick={() => onDecide(i, "accepted")}
                            className="rounded px-1 text-xs text-accepted hover:bg-accepted-soft"
                            title="Accept (a)"
                          >
                            ✓
                          </button>
                          <button
                            onClick={() => onDecide(i, "rejected")}
                            className="rounded px-1 text-xs text-rejected hover:bg-surface-sunken"
                            title="Reject (r)"
                          >
                            ✕
                          </button>
                        </span>
                      </li>
                    );
                  })}
                </ul>
              )}
            </div>
          );
        })}
      </div>
      <div className="border-t border-border px-4 py-2 text-[11px] text-muted">
        j/k walk · a accept · r reject · A/R whole rule · u undo
      </div>
    </aside>
  );
}
