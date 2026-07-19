import { useState } from "react";
import { toggleTheme } from "../theme";
import type { RulesInfo } from "../App";

interface Props {
  rulesInfo: RulesInfo | null;
}

export function TopBar({ rulesInfo }: Props) {
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
        <button
          onClick={() => setTheme(toggleTheme())}
          className="rounded-md border border-border px-2.5 py-1 text-sm text-muted hover:bg-surface-sunken"
          title="Toggle theme"
        >
          {theme === "dark" ? "☀" : "☾"}
        </button>
      </div>
    </header>
  );
}
