import {
  forwardRef,
  memo,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
} from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { ScanOutcome } from "../App";
import type { Review } from "../review";

export type ViewMode = "before" | "after";

interface Props {
  outcome: ScanOutcome | null;
  review: Review;
  mode: ViewMode;
  dragOver: boolean;
  searchQuery: string;
  /** Line index of the current search match, for the row tint. */
  currentSearchLine: number | null;
  onBrowse: () => void;
}

export interface DocumentViewHandle {
  /** Scroll the given line into view. Used for search-match jumps, which
   *  live in App (it owns `matches`/`currentMatch`) rather than here. */
  scrollToLine: (line: number) => void;
}

/**
 * Pixel height of one document row: font-mono text-[13px] leading-6 = 24px.
 * Single source of truth for the virtualizer's row-size estimate. If the
 * row's type scale ever becomes user-adjustable (e.g. an accessibility
 * text-size setting), this is the one constant that needs to become a
 * function of that setting — the virtualizer itself already takes
 * `estimateSize` as a callback, so it doesn't need to change shape, just
 * what it reads.
 */
const ROW_HEIGHT_PX = 24;

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

// memo + forwardRef: App re-renders on every keystroke and every match
// step. Before virtualization this required imperatively mutating the DOM
// (classList/scrollIntoView) to avoid redrawing 30k rows on every such
// render. Now that only the visible rows are ever mounted, re-rendering on
// prop changes (e.g. currentSearchLine) is cheap — so this version renders
// declaratively from props instead, and just exposes scrollToLine for the
// one thing App still needs to trigger imperatively (a scroll jump).
export const DocumentView = memo(
  forwardRef<DocumentViewHandle, Props>(function DocumentView(
    { outcome, review, mode, dragOver, searchQuery, currentSearchLine, onBrowse },
    ref,
  ) {
    const parentRef = useRef<HTMLDivElement | null>(null);

    const rowVirtualizer = useVirtualizer({
      count: outcome ? outcome.lines.length : 0,
      getScrollElement: () => parentRef.current,
      estimateSize: () => ROW_HEIGHT_PX,
      overscan: 20,
    });

    const ruleIds = useMemo(
      () =>
        outcome
          ? [...new Set(outcome.findings.map((f) => f.rule_id))].sort()
          : [],
      [outcome],
    );

    // Reverse index: which line each finding lives on. Lets us scroll to
    // the focused finding without a DOM query — the row it's on may not
    // even be mounted while virtualized, so there's nothing to query.
    const findingLine = useMemo(() => {
      if (!outcome) return [] as number[];
      const map: number[] = new Array(outcome.findings.length);
      outcome.lines.forEach((segments, lineIdx) => {
        for (const seg of segments) {
          if (seg.finding !== null) map[seg.finding] = lineIdx;
        }
      });
      return map;
    }, [outcome]);

    // Widest line in the document, in characters. Monospace + CSS `ch`
    // units give the exact pixel width without measuring anything in the
    // DOM — and, importantly, give the row wrapper an explicit width.
    // Absolutely-positioned virtual rows don't otherwise contribute to
    // their parent's intrinsic width the way normal-flow content does, so
    // without this the outer pane's horizontal scrollbar would stop
    // reflecting the true width of long lines.
    const maxLineChars = useMemo(() => {
      if (!outcome) return 0;
      let max = 0;
      for (const segments of outcome.lines) {
        let len = 0;
        for (const seg of segments) len += seg.text.length;
        if (len > max) max = len;
      }
      return max;
    }, [outcome]);

    useImperativeHandle(
      ref,
      () => ({
        scrollToLine: (line: number) => {
          rowVirtualizer.scrollToIndex(line, { align: "center" });
        },
      }),
      [rowVirtualizer],
    );

    // Keep the focused finding's row in view as j/k (or a sidebar click)
    // moves focus. Replaces the old focusedRef + scrollIntoView, which
    // only worked because every row was always mounted.
    useEffect(() => {
      if (mode !== "before" || review.focused === null) return;
      const line = findingLine[review.focused];
      if (line === undefined) return;
      rowVirtualizer.scrollToIndex(line, { align: "center" });
    }, [review.focused, mode, findingLine, rowVirtualizer]);

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

    // Captured into its own const so TypeScript's null-narrowing survives
    // into renderLine below (narrowing doesn't propagate into nested
    // function bodies through a destructured prop on its own).
    const oc = outcome;

    /** Render one line's content for the current view mode. */
    function renderLine(i: number) {
      const segments = oc.lines[i];

      if (mode === "after") {
        return segments.map((seg, j) => {
          const isAccepted =
            seg.finding !== null && review.states[seg.finding] === "accepted";
          return isAccepted ? (
            <span key={j} className="text-muted">
              [REDACTED:{oc.findings[seg.finding!].rule_id}]
            </span>
          ) : (
            <span key={j}>
              {highlightQuery(seg.text, searchQuery, `${i}-${j}`)}
            </span>
          );
        });
      }

      return segments.map((seg, j) => {
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
              className="rounded-sm bg-accepted-soft px-1 font-medium text-accepted"
              style={focusRing}
              title={`accepted · was: ${seg.text}`}
            >
              [REDACTED:{oc.findings[idx].rule_id}]
            </mark>
          );
        }

        if (state === "rejected") {
          return (
            <span
              key={j}
              className="rounded-sm px-0.5 [text-decoration:underline_dotted] decoration-rejected"
              style={focusRing}
              title={`rejected · ${oc.findings[idx].rule_id}`}
            >
              {seg.text}
            </span>
          );
        }

        return (
          <mark
            key={j}
            className="rounded-sm bg-pending-soft px-0.5 text-text"
            style={{
              boxShadow: `inset 0 -2px 0 var(--rule-${ruleHue(
                oc.findings[idx].rule_id,
                ruleIds,
              )})`,
              ...focusRing,
            }}
            title={`pending · ${oc.findings[idx].rule_id}`}
          >
            {seg.text}
          </mark>
        );
      });
    }

    return (
      <section
        ref={parentRef}
        className="h-full overflow-auto bg-surface-sunken font-mono text-[13px] leading-6"
      >
        {mode === "after" && (
          <div className="sticky top-0 z-[1] border-b border-border bg-surface-raised px-4 py-1 text-xs text-muted">
            Preview: output as it would export now · pending findings are NOT
            redacted until accepted
          </div>
        )}
        <div
          style={{
            position: "relative",
            height: rowVirtualizer.getTotalSize(),
            width: `${Math.max(maxLineChars, 1)}ch`,
            minWidth: "100%",
          }}
        >
          {rowVirtualizer.getVirtualItems().map((item) => (
            <div
              key={item.key}
              data-line={item.index}
              className={`flex ${
                currentSearchLine === item.index ? "search-current" : ""
              }`}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: item.size,
                transform: `translateY(${item.start}px)`,
              }}
            >
              <span className="w-12 shrink-0 select-none pr-3 text-right text-muted/60">
                {item.index + 1}
              </span>
              <span className="whitespace-pre">{renderLine(item.index)}</span>
            </div>
          ))}
        </div>
      </section>
    );
  }),
);
