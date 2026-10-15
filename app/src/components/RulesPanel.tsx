import { memo, useState } from "react";
import type {
  PatternPreview,
  PatternSyntax,
  RuleView,
  SaveRuleResult,
} from "../App";

interface Props {
  rules: RuleView[];
  rulesPath: string | null;
  previewPattern: string;
  previewSyntax: PatternSyntax;
  onPreviewSyntaxChange: (syntax: PatternSyntax) => void;
  preview: PatternPreview | null;
  previewError: string | null;
  onPreviewPatternChange: (pattern: string) => void;
  onSaveRule: (id: string, name: string) => Promise<SaveRuleResult>;
  hasDocument: boolean;
}

/**
 * The active rule set, a pattern tester, and saving a tested pattern as a
 * rule in the active rules file (ADR 004).
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
  onSaveRule,
  hasDocument,
}: Props) {
  const [ruleId, setRuleId] = useState("");
  const [ruleName, setRuleName] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

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
  const canSave =
    previewPattern !== "" &&
    previewError === null &&
    ruleId.trim() !== "" &&
    ruleName.trim() !== "" &&
    !saving;

  const save = async () => {
    setSaving(true);
    const result = await onSaveRule(ruleId.trim(), ruleName.trim());
    setSaving(false);
    switch (result.status) {
      case "saved":
        setRuleId("");
        setRuleName("");
        setSaveError(null);
        break;
      case "cancelled":
        break;
      case "error":
        setSaveError(result.message);
        break;
    }
  };

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

        {previewPattern !== "" && previewError === null && (
          <div className="mt-2 border-t border-border pt-2">
            <div className="flex gap-1.5">
              <input
                value={ruleId}
                onChange={(e) => setRuleId(e.target.value)}
                placeholder="id, e.g. badge_number"
                spellCheck={false}
                aria-label="Rule id"
                className="w-1/2 rounded-md border border-border bg-surface-sunken px-2 py-1 font-mono text-xs outline-none focus:border-accent focus:ring-2 focus:ring-accent"
              />
              <input
                value={ruleName}
                onChange={(e) => setRuleName(e.target.value)}
                placeholder="Name"
                aria-label="Rule name"
                className="w-1/2 rounded-md border border-border bg-surface-sunken px-2 py-1 text-xs outline-none focus:border-accent focus:ring-2 focus:ring-accent"
              />
            </div>
            <div className="mt-1.5 flex items-center justify-between gap-2">
              <span className="truncate text-[11px] text-muted">
                Saves to {fileName ?? "a new rules file"}
              </span>
              <button
                onClick={() => void save()}
                disabled={!canSave}
                className="shrink-0 rounded-md bg-accent px-2 py-1 text-xs font-medium text-surface-raised hover:opacity-90 disabled:opacity-40"
              >
                {saving ? "Saving…" : "Save as rule"}
              </button>
            </div>
            {saveError && (
              <div className="mt-1 whitespace-pre-wrap wrap-break-word text-xs text-pending">
                {saveError}
              </div>
            )}
          </div>
        )}
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
