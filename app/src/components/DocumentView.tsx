import type { ScanOutcome } from "../App";

interface Props {
  outcome: ScanOutcome | null;
  dragOver: boolean;
  onBrowse: () => void;
}

export function DocumentView({ outcome, dragOver, onBrowse }: Props) {
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

  const lines = outcome.text.split("\n");
  return (
    <section className="h-full overflow-auto bg-surface-sunken font-mono text-[13px] leading-6">
      <div className="min-w-max px-0 py-2">
        {lines.map((line, i) => (
          <div key={i} className="flex">
            <span className="w-12 shrink-0 select-none pr-3 text-right text-muted/60">
              {i + 1}
            </span>
            <span className="whitespace-pre">{line}</span>
          </div>
        ))}
      </div>
    </section>
  );
}
