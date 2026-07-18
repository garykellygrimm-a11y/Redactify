import { useEffect, useMemo, useRef } from "react";
import type { ScanOutcome } from "../App";
import type { Review } from "../review";

interface Props {
  outcome: ScanOutcome | null;
  review: Review;
  dragOver: boolean;
  onBrowse: () => void;
}

/** Stable rule_id -> palette slot (1-5). Same rule, same hue, every file. */
export function ruleHue(ruleId: string, ruleIds: string[]): number {
  return (ruleIds.indexOf(ruleId) % 5) + 1;
}

export function DocumentView({ outcome, review, dragOver, onBrowse }: Props) {
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
  }, [review.focused]);

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
              {segments.map((seg, j) => {
                if (seg.finding === null)
                  return <span key={j}>{seg.text}</span>;

                const idx = seg.finding;
                const state = review.states[idx];
                const focused = idx === review.focused;
                const focusRing = focused
                  ? { outline: "2px solid var(--accent)", outlineOffset: "1px" }
                  : {};

                // ACCEPTED: the live preview — text collapses to its token.
                if (state === "accepted") {
                  return (
                    <mark
                      key={j}
                      ref={focused ? focusedRef : null}
                      className="rounded-sm bg-accepted-soft px-1 font-medium text-accepted"
                      style={focusRing}
                      title={`accepted · was: ${seg.text}`}
                    >
                      [REDACTED:{outcome.findings[idx].rule_id}]
                    </mark>
                  );
                }

                // REJECTED: plain text, dotted underline keeps it findable.
                if (state === "rejected") {
                  return (
                    <span
                      key={j}
                      ref={focused ? focusedRef : null}
                      className="rounded-sm px-0.5 [text-decoration:underline_dotted] decoration-rejected"
                      style={focusRing}
                      title={`rejected · ${outcome.findings[idx].rule_id}`}
                    >
                      {seg.text}
                    </span>
                  );
                }

                // PENDING: amber, awaiting judgment.
                return (
                  <mark
                    key={j}
                    ref={focused ? focusedRef : null}
                    className="rounded-sm bg-pending-soft px-0.5 text-text"
                    style={{
                      boxShadow: `inset 0 -2px 0 var(--rule-${ruleHue(
                        outcome.findings[idx].rule_id,
                        ruleIds,
                      )})`,
                      ...focusRing,
                    }}
                    title={`pending · ${outcome.findings[idx].rule_id}`}
                  >
                    {seg.text}
                  </mark>
                );
              })}
            </span>
          </div>
        ))}
      </div>
    </section>
  );
}
