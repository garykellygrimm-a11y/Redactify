import { useEffect, useMemo, useRef } from "react";
import type { ScanOutcome } from "../App";

interface Props {
  outcome: ScanOutcome | null;
  dragOver: boolean;
  focusedFinding: number | null;
  onBrowse: () => void;
}

/** Stable rule_id -> palette slot (1-5). Same rule, same hue, every file. */
export function ruleHue(ruleId: string, ruleIds: string[]): number {
  return (ruleIds.indexOf(ruleId) % 5) + 1;
}

export function DocumentView({
  outcome,
  dragOver,
  focusedFinding,
  onBrowse,
}: Props) {
  const focusedRef = useRef<HTMLElement | null>(null);

  const ruleIds = useMemo(
    () =>
      outcome
        ? [...new Set(outcome.findings.map((f) => f.rule_id))].sort()
        : [],
    [outcome],
  );

  useEffect(() => {
    focusedRef.current?.scrollIntoView({ block: "center", behavior: "smooth" });
  }, [focusedFinding]);

  if (!outcome) {
    return (
      <section
        className={`flex h-full items-center justify-center bg-surface-sunken transition-colors ${
          dragOver ? "bg-accent-soft" : ""
        }`}
      >
        <div className="max-w-sm text-center">
          <div className="mb-3 text-4xl">⬇</div>
          <p className="font-medium">
            {dragOver ? "Release to scan" : "Drop a file to scan it"}
          </p>
          <p className="mt-1 text-sm text-muted">
            or use <span className="font-medium">Browse</span> — your files
            never leave this machine
          </p>
          <button
            onClick={onBrowse}
            className="mt-4 rounded-md bg-accent px-4 py-1.5 text-sm font-medium text-surface-raised hover:opacity-90"
          >
            Browse…
          </button>
        </div>
      </section>
    );
  }

  return (
    <section className="h-full overflow-auto bg-surface-sunken font-mono text-[13px] leading-6">
      <div className="min-w-max px-0 py-2">
        {outcome.lines.map((segments, i) => (
          <div key={i} className="flex">
            <span className="w-12 shrink-0 select-none pr-3 text-right text-muted/60">
              {i + 1}
            </span>
            <span className="whitespace-pre">
              {segments.map((seg, j) =>
                seg.finding === null ? (
                  <span key={j}>{seg.text}</span>
                ) : (
                  <mark
                    key={j}
                    ref={seg.finding === focusedFinding ? focusedRef : null}
                    className="rounded-sm px-0.5"
                    style={{
                      background: `color-mix(in srgb, var(--rule-${ruleHue(
                        outcome.findings[seg.finding].rule_id,
                        ruleIds,
                      )}) 22%, transparent)`,
                      color: "inherit",
                      outline:
                        seg.finding === focusedFinding
                          ? "2px solid var(--accent)"
                          : "none",
                    }}
                    title={outcome.findings[seg.finding].rule_id}
                  >
                    {seg.text}
                  </mark>
                ),
              )}
            </span>
          </div>
        ))}
      </div>
    </section>
  );
}
