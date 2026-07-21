import { memo, useEffect, useMemo, useRef } from "react";
import type { ScanOutcome } from "../App";
import type { Review } from "../review";

export type ViewMode = "before" | "after";

interface Props {
  outcome: ScanOutcome | null;
  review: Review;
  mode: ViewMode;
  dragOver: boolean;
  searchQuery: string;
  onBrowse: () => void;
}

/** Stable rule_id -> palette slot (1-5). Same rule, same hue, every file. */
export function ruleHue(ruleId: string, ruleIds: string[]): number {
  return (ruleIds.indexOf(ruleId) % 5) + 1;
}

/** Render plain text with case-insensitive `query` occurrences marked. */
function highlightQuery(text: string, query: string, keyBase: string) {
  if (!query) return text;
  const lower = text.toLowerCase();
  const q = query.toLowerCase();
  const nodes: React.ReactNode[] = [];
  let pos = 0;
  let hit = lower.indexOf(q, pos);
  if (hit === -1) return text;
  while (hit !== -1) {
    if (hit > pos) nodes.push(text.slice(pos, hit));
    nodes.push(
      <mark
        key={`${keyBase}-${hit}`}
        className="rounded-sm bg-accent-soft px-0 text-inherit"
      >
        {text.slice(hit, hit + q.length)}
      </mark>,
    );
    pos = hit + q.length;
    hit = lower.indexOf(q, pos);
  }
  if (pos < text.length) nodes.push(text.slice(pos));
  return nodes;
}

// memo: App re-renders on every keystroke and every match step; without
// this, all 30k rows redraw each time. The current-match row tint is
// applied imperatively from App via .search-current + data-line, so
// stepping matches changes NO props here and re-renders nothing.
export const DocumentView = memo(function DocumentView({
  outcome,
  review,
  mode,
  dragOver,
  searchQuery,
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

  // "auto" (instant) rather than "smooth": animating a jump across tens
  // of thousands of rows forces the webview to paint the whole journey.
  useEffect(() => {
    focusedRef.current?.scrollIntoView({ block: "center", behavior: "auto" });
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

  // AFTER: read-only preview of the export as it stands.
  if (mode === "after") {
    return (
      <section className="h-full overflow-auto bg-surface-sunken font-mono text-[13px] leading-6">
        <div className="sticky top-0 z-[1] border-b border-border bg-surface-raised px-4 py-1 text-xs text-muted">
          Preview: output as it would export now · pending findings are NOT
          redacted until accepted
        </div>
        <div className="min-w-max px-0 py-2">
          {outcome.lines.map((segments, i) => (
            <div key={i} data-line={i} className="flex">
              <span className="w-12 shrink-0 select-none pr-3 text-right text-muted/60">
                {i + 1}
              </span>
              <span className="whitespace-pre">
                {segments.map((seg, j) => {
                  const isAccepted =
                    seg.finding !== null &&
                    review.states[seg.finding] === "accepted";
                  return isAccepted ? (
                    <span key={j} className="text-muted">
                      [REDACTED:{outcome.findings[seg.finding!].rule_id}]
                    </span>
                  ) : (
                    <span key={j}>
                      {highlightQuery(seg.text, searchQuery, `${i}-${j}`)}
                    </span>
                  );
                })}
              </span>
            </div>
          ))}
        </div>
      </section>
    );
  }

  return (
    <section className="h-full overflow-auto bg-surface-sunken font-mono text-[13px] leading-6">
      <div className="min-w-max px-0 py-2">
        {outcome.lines.map((segments, i) => (
          <div key={i} data-line={i} className="flex">
            <span className="w-12 shrink-0 select-none pr-3 text-right text-muted/60">
              {i + 1}
            </span>
            <span className="whitespace-pre">
              {segments.map((seg, j) => {
                if (seg.finding === null)
                  return (
                    <span key={j}>
                      {highlightQuery(seg.text, searchQuery, `${i}-${j}`)}
                    </span>
                  );

                const idx = seg.finding;
                const state = review.states[idx];
                const focused = idx === review.focused;
                const focusRing = focused
                  ? { outline: "2px solid var(--accent)", outlineOffset: "1px" }
                  : {};

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
});
