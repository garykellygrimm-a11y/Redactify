export function DocumentView() {
  return (
    <section className="flex h-full items-center justify-center bg-surface-sunken">
      <div className="max-w-sm text-center">
        <div className="mb-3 text-4xl">⬇</div>
        <p className="font-medium">Drop a file to scan it</p>
        <p className="mt-1 text-sm text-muted">
          or use <span className="font-medium">Browse</span> — your files never
          leave this machine
        </p>
        <button
          className="mt-4 rounded-md bg-accent px-4 py-1.5 text-sm font-medium text-surface-raised hover:opacity-90"
          disabled
          title="Wired up in the next step"
        >
          Browse…
        </button>
      </div>
    </section>
  );
}
