import { useState } from "react";
import type {
  PatternPreview,
  PatternSyntax,
  RuleView,
  ScanOutcome,
} from "../App";
import type { Review, Verdict } from "../review";
import { FindingsPanel } from "./FindingsPanel";
import { RulesPanel } from "./RulesPanel";

interface Props {
  outcome: ScanOutcome | null;
  review: Review;
  rules: RuleView[];
  rulesPath: string | null;
  previewPattern: string;
  previewSyntax: PatternSyntax;
  onPreviewSyntaxChange: (syntax: PatternSyntax) => void;
  preview: PatternPreview | null;
  previewError: string | null;
  onPreviewPatternChange: (pattern: string) => void;
  onFocus: (index: number) => void;
  onDecide: (index: number, verdict: Verdict) => void;
  onDecideGroup: (indices: number[], verdict: Verdict) => void;
}

type Tab = "findings" | "rules";

/**
 * Thin tab container. The panels live in their own files: FindingsPanel is
 * memoized and expensive (it groups every finding on each render), while
 * this shell is cheap and re-renders freely.
 *
 * Rules live here in the sidebar rather than in a modal on purpose. The
 * editor arriving in the next stage highlights matches live in the open
 * document, which means the pattern being edited and the document have to
 * be visible at the same time — a full-screen modal like HelpPanel would
 * cover exactly what you need to watch.
 */
export function Sidebar({
  outcome,
  review,
  rules,
  rulesPath,
  previewPattern,
  previewSyntax,
  onPreviewSyntaxChange,
  preview,
  previewError,
  onPreviewPatternChange,
  onFocus,
  onDecide,
  onDecideGroup,
}: Props) {
  const [tab, setTab] = useState<Tab>("findings");

  return (
    <aside className="flex h-full flex-col bg-surface-raised">
      <div className="flex shrink-0 border-b border-border">
        {(["findings", "rules"] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`flex-1 px-4 py-2 text-sm capitalize ${
              tab === t
                ? "border-b-2 border-accent font-medium text-accent"
                : "text-muted hover:bg-surface-sunken"
            }`}
          >
            {t}
            {t === "rules" && rules.length > 0 && (
              <span className="ml-1.5 text-xs text-muted">{rules.length}</span>
            )}
          </button>
        ))}
      </div>

      <div className="min-h-0 flex-1">
        {tab === "findings" ? (
          <FindingsPanel
            outcome={outcome}
            review={review}
            onFocus={onFocus}
            onDecide={onDecide}
            onDecideGroup={onDecideGroup}
          />
        ) : (
          <RulesPanel
            rules={rules}
            rulesPath={rulesPath}
            previewPattern={previewPattern}
            previewSyntax={previewSyntax}
            onPreviewSyntaxChange={onPreviewSyntaxChange}
            preview={preview}
            previewError={previewError}
            onPreviewPatternChange={onPreviewPatternChange}
            hasDocument={outcome !== null}
          />
        )}
      </div>
    </aside>
  );
}
