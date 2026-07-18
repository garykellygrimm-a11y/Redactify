interface Props {
  result: {
    output_path: string;
    manifest_path: string;
    source_sha256: string;
    output_sha256: string;
    applied_count: number;
    rejected_count: number;
  };
  onDismiss: () => void;
}

function HashRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center gap-2">
      <span className="w-28 shrink-0 text-xs text-muted">{label}</span>
      <code className="min-w-0 flex-1 truncate rounded bg-surface-sunken px-2 py-1 font-mono text-xs">
        {value}
      </code>
      <button
        onClick={() => void navigator.clipboard.writeText(value)}
        className="shrink-0 rounded border border-border px-2 py-1 text-xs text-muted hover:bg-surface-sunken"
        title="Copy"
      >
        Copy
      </button>
    </div>
  );
}

export function ExportSuccess({ result, onDismiss }: Props) {
  return (
    <div className="fixed inset-0 z-10 flex items-center justify-center bg-black/40">
      <div className="w-[560px] max-w-[90vw] rounded-lg border border-border bg-surface-raised p-6 shadow-xl">
        <div className="mb-1 text-lg font-semibold text-accepted">
          ✓ Export complete
        </div>
        <p className="mb-4 text-sm text-muted">
          {result.applied_count} redaction
          {result.applied_count === 1 ? "" : "s"} applied ·{" "}
          {result.rejected_count} finding
          {result.rejected_count === 1 ? "" : "s"} left as-is by review
        </p>
        <div className="mb-4 space-y-2">
          <HashRow label="Source SHA-256" value={result.source_sha256} />
          <HashRow label="Output SHA-256" value={result.output_sha256} />
        </div>
        <div className="mb-5 space-y-1 text-xs text-muted">
          <div className="truncate" title={result.output_path}>
            Redacted file: <code>{result.output_path}</code>
          </div>
          <div className="truncate" title={result.manifest_path}>
            Audit manifest: <code>{result.manifest_path}</code>
          </div>
        </div>
        <p className="mb-4 text-xs text-muted">
          Anyone holding the original can verify this export independently:{" "}
          <code>sha256sum</code> of source and output must match the values
          above and in the manifest.
        </p>
        <div className="flex justify-end">
          <button
            onClick={onDismiss}
            className="rounded-md bg-accent px-4 py-1.5 text-sm font-medium text-surface-raised hover:opacity-90"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
