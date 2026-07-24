const STORAGE_KEY = "redactify-text-size";

export type TextSize = "small" | "medium" | "large";

const LEVELS: TextSize[] = ["small", "medium", "large"];

/**
 * Font size and row height per level. DocumentView's virtualizer reads
 * `rowPx` directly for its row-size estimate, and the same numbers drive
 * the --doc-font-size / --doc-row-height CSS custom properties — one
 * table, two consumers, so they can't drift apart the way two separately
 * hand-tuned constants could.
 */
export const TEXT_SIZE_METRICS: Record<TextSize, { fontPx: number; rowPx: number }> = {
  small: { fontPx: 12, rowPx: 20 },
  medium: { fontPx: 13, rowPx: 24 },
  large: { fontPx: 16, rowPx: 28 },
};

function isTextSize(value: string): value is TextSize {
  return (LEVELS as string[]).includes(value);
}

export function loadTextSize(): TextSize {
  const stored = window.localStorage.getItem(STORAGE_KEY);
  return stored && isTextSize(stored) ? stored : "medium";
}

export function saveTextSize(size: TextSize) {
  window.localStorage.setItem(STORAGE_KEY, size);
}

/** Push a level's metrics onto <html> as CSS custom properties. */
export function applyTextSize(size: TextSize) {
  const { fontPx, rowPx } = TEXT_SIZE_METRICS[size];
  document.documentElement.dataset.textSize = size;
  document.documentElement.style.setProperty("--doc-font-size", `${fontPx}px`);
  document.documentElement.style.setProperty("--doc-row-height", `${rowPx}px`);
}

/** Pure step function — clamps at the ends rather than wrapping. */
export function stepTextSize(current: TextSize, direction: 1 | -1): TextSize {
  const idx = LEVELS.indexOf(current);
  return LEVELS[Math.min(LEVELS.length - 1, Math.max(0, idx + direction))];
}
