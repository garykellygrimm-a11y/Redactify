import { memo } from "react";
import type { PatternPreview, PatternSyntax, RuleView } from "../App";

interface Props {
  rules: RuleView[];
  rulesPath: string | null;
  previewPattern: string;
  previewSyntax: PatternSyntax;
  onPreviewSyntaxChange: (syntax: PatternSyntax) => void;
  preview: PatternPreview | null;
  previewError: string | null;
  onPreviewPatternChange: (pattern: string) => void;
  hasDocument: boolean;
}

/**
 * Read-only listing of the active rule set. Editing arrives in a later
 * stage; the first thing needed is simply being able to SEE what's
 * running, which until now the app couldn't do at all — it knew only a
 * file path and a count.
 *
 * Not memoized, unlike FindingsPanel: this list is tens of rows and
 * changes only when a rules file is loaded, so there's nothing to protect
 * against here.
 */
export const RulesPanel = memo(function RulesPanel({
  rules,
  rulesPath,
  previewPattern,
  previewSyntax,
  onPreviewSyntaxChange,
  preview,
  previewError,
  onPreviewPatternChange,
  hasDocument,
}: Props) {
  if (rules.length === 0) {
    return (
      <div className="flex h-full flex-col overflow-y-auto">
        <div className="flex flex-1 items-center justify-center p-6 text-center text-sm text-muted">
          Loading rules…
        </div>
      </div>
    );
  }

  const userCount = rules.filter((r) => r.source === "user").length;
  const validatedCount = rules.filter((r) => r.validated).length;
  const fileName = rulesPath?.replace(/^.*[\\/]/, "");

  return (
    <div className="flex h-full flex-col overflow-y-auto">
      <div className="border-b border-border p-3">
        <div className="mb-1 flex items-center justify-between">
          <label className="text-xs font-medium text-muted">Test a pattern</label>
          <div className="flex overflow-hidden rounded border border-border text-[10px]">
            {(["glob", "regex"] as const).map((mode) => (
              <button
                key={mode}
                onClick={() => onPreviewSyntaxChange(mode)}
                className={`px-1.5 py-0.5 ${
                  previewSyntax === mode
                    ? "bg-accent-soft text-accent"
                    : "text-muted hover:bg-surface-sunken"
                }`}
              >
                {mode === "glob" ? "Simple" : "Regex"}
              </button>
            ))}
          </div>
        </div>
        <input
          value={previewPattern}
          onChange={(e) => onPreviewPatternChange(e.target.value)}
          placeholder={
            previewSyntax === "glob"
              ? "*@google.com"
              : String.raw`\b\d{3}-\d{2}-\d{4}\b`
          }
          spellCheck={false}
          className="w-full rounded-md border border-border bg-surface-sunken px-2 py-1 font-mono text-xs outline-none focus:border-accent focus:ring-2 focus:ring-accent"
        />
        {previewSyntax === "glob" && preview && previewPattern && !previewError && (
          <div
            className="mt-1 truncate font-mono text-[11px] text-muted"
            title={preview.regex}
          >
            → {preview.regex}
          </div>
        )}

        <div className="mt-1.5 min-h-4 text-xs">
          {previewError ? (
            <span className="text-pending">{previewError}</span>
          ) : !previewPattern ? (
            <span className="text-muted">
              {previewSyntax === "glob"
                ? "Use * for any run of characters, ? for one."
                : "Matches highlight in the document as you type."}
            </span>
          ) : !hasDocument ? (
            <span className="text-muted">Open a document to see matches.</span>
          ) : preview ? (
            <span className={preview.match_count > 0 ? "text-accent" : "text-muted"}>
              {preview.match_count} match{preview.match_count === 1 ? "" : "es"}
              {preview.truncated && " (highlighting first 200 lines)"}
            </span>
          ) : (
            <span className="text-muted">Checking…</span>
          )}
        </div>
      </div>

      <div className="border-b border-border px-4 py-3">
        <div className="text-sm font-medium">
          {rules.length} rule{rules.length === 1 ? "" : "s"} active
        </div>
        <div className="mt-0.5 text-xs text-muted">
          {userCount > 0
            ? `${rules.length - userCount} builtin · ${userCount} from ${fileName ?? "your rules file"}`
            : `all builtin · ${validatedCount} checksum-validated`}
        </div>
      </div>

      <div className="p-2">
        {rules.map((rule) => (
          <div
            key={rule.id}
            className="mb-1 rounded-md px-2 py-1.5 hover:bg-surface-sunken"
          >
            <div className="flex items-center gap-2">
              <span className="truncate font-mono text-[13px]">{rule.id}</span>
              {rule.source === "user" && (
                <span
                  className="shrink-0 rounded bg-accent-soft px-1.5 text-[10px] text-accent"
                  title="From your rules file. A user rule sharing a builtin's id replaces it."
                >
                  user
                </span>
              )}
              {rule.validated && (
                <span
                  className="shrink-0 rounded bg-accepted-soft px-1.5 text-[10px] text-accepted"
                  title="Runs a real check beyond the pattern — a checksum or structural validation — so it produces far fewer false positives than shape-matching alone."
                >
                  validated
                </span>
              )}
              {rule.finding_group !== null && (
                <span
                  className="shrink-0 rounded bg-pending-soft px-1.5 text-[10px] text-pending"
                  title={`Only capture group ${rule.finding_group} becomes the finding — the rest of the match is context used for precision.`}
                >
                  partial
                </span>
              )}
            </div>
            <div className="mt-0.5 text-xs text-muted">{rule.name}</div>
            <div
              className="mt-1 truncate font-mono text-[11px] text-muted/70"
              title={rule.pattern}
            >
              {rule.pattern}
            </div>
          </div>
        ))}
      </div>

      <div className="border-t border-border px-4 py-2 text-[11px] text-muted">
        Load your own with File → Load Rules (Ctrl+L)
      </div>
    </div>
  );
});
