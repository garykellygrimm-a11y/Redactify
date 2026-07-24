import { useState } from "react";
import { toggleTheme } from "../theme";
import type { TextSize } from "../textSize";
import type { RulesInfo } from "../App";

interface Props {
  rulesInfo: RulesInfo | null;
  textSize: TextSize;
  onChangeTextSize: (direction: 1 | -1) => void;
  onOpenHelp: () => void;
}

const SIZE_LABEL: Record<TextSize, string> = {
  small: "S",
  medium: "M",
  large: "L",
};

export function TopBar({
  rulesInfo,
  textSize,
  onChangeTextSize,
  onOpenHelp,
}: Props) {
  const [theme, setTheme] = useState(
    document.documentElement.dataset.theme ?? "light",
  );

  const rulesFile = rulesInfo?.path.replace(/^.*[\\/]/, "");

  return (
    <header className="flex h-12 shrink-0 items-center gap-3 border-b border-border bg-surface-raised px-4">
      <span className="font-semibold tracking-tight">Redactify</span>
      <span className="rounded-full bg-accent-soft px-2 py-0.5 text-xs text-accent">
        100% offline
      </span>
      {rulesInfo && (
        <span
          className="rounded-full border border-border px-2 py-0.5 text-xs text-muted"
          title={`${rulesInfo.path} · ${rulesInfo.count} rules active`}
        >
          rules: {rulesFile} · {rulesInfo.count}
        </span>
      )}
      <div className="ml-auto flex items-center gap-2">
        <div className="flex items-center overflow-hidden rounded-md border border-border">
          <button
            onClick={() => onChangeTextSize(-1)}
            disabled={textSize === "small"}
            className="px-2 py-1 text-xs text-muted hover:bg-surface-sunken disabled:opacity-30"
            title="Decrease text size"
          >
            A−
          </button>
          <span className="border-x border-border px-1.5 py-1 text-xs text-muted">
            {SIZE_LABEL[textSize]}
          </span>
          <button
            onClick={() => onChangeTextSize(1)}
            disabled={textSize === "large"}
            className="px-2 py-1 text-xs text-muted hover:bg-surface-sunken disabled:opacity-30"
            title="Increase text size"
          >
            A+
          </button>
        </div>
        <button
          onClick={() => setTheme(toggleTheme())}
          className="rounded-md border border-border px-2.5 py-1 text-sm text-muted hover:bg-surface-sunken"
          title="Toggle theme"
        >
          {theme === "dark" ? "☀" : "☾"}
        </button>
        <button
          onClick={onOpenHelp}
          className="rounded-md border border-border px-2.5 py-1 text-sm text-muted hover:bg-surface-sunken"
          title="Keyboard shortcuts (?)"
        >
          ?
        </button>
      </div>
    </header>
  );
}
