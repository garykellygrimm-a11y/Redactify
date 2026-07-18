import type { ScanOutcome } from "../App";
import type { Review } from "../review";
import { tally } from "../review";

interface Props {
  review: Review;
  outcome: ScanOutcome | null;
  onExport: () => void;
}

export function VerdictStrip({ review, outcome, onExport }: Props) {
  if (!outcome) {
    return (
      <footer className="flex h-11 shrink-0 items-center gap-4 border-t border-border bg-surface-raised px-4 text-sm">
        <span className="text-muted">No findings yet</span>
        <button
          className="ml-auto rounded-md bg-accent px-4 py-1 font-medium text-surface-raised disabled:opacity-40"
          disabled
        >
          Export
        </button>
      </footer>
    );
  }

  const { accepted, rejected, pending } = tally(review);
  const ready = pending === 0 && review.states.length > 0;

  return (
    <footer className="flex h-11 shrink-0 items-center gap-4 border-t border-border bg-surface-raised px-4 text-sm">
      <span className="text-accepted">{accepted} accepted</span>
      <span className="text-rejected">{rejected} rejected</span>
      <span className={pending > 0 ? "font-medium text-pending" : "text-muted"}>
        {pending} pending
      </span>
      <button
        onClick={onExport}
        disabled={!ready}
        className="ml-auto rounded-md bg-accent px-4 py-1 font-medium text-surface-raised transition-opacity disabled:opacity-40"
        title={
          ready
            ? "Write redacted file + manifest"
            : "Export unlocks when every finding is decided"
        }
      >
        Export
      </button>
    </footer>
  );
}
