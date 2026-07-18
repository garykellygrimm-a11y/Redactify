// Review state as an event log: an append-only list of decisions,
// with current state derived by replay. The log IS the undo feature
// (undo = pop), and at export time it becomes the payload Rust
// receives. Rust stays uninvolved until export — no IPC per keypress.

export type Verdict = "accepted" | "rejected";
export type ReviewState = Verdict | "pending";

export interface ReviewEvent {
  /** Indices into ScanOutcome.findings this event decided. */
  findings: number[];
  verdict: Verdict;
}

export interface Review {
  log: ReviewEvent[];
  /** Derived: one state per finding, same order as findings. */
  states: ReviewState[];
  /** The finding the keyboard acts on. */
  focused: number | null;
}

export function emptyReview(findingCount: number): Review {
  return {
    log: [],
    states: Array(findingCount).fill("pending"),
    focused: findingCount > 0 ? 0 : null,
  };
}

function replay(log: ReviewEvent[], findingCount: number): ReviewState[] {
  const states: ReviewState[] = Array(findingCount).fill("pending");
  for (const event of log) {
    for (const i of event.findings) states[i] = event.verdict;
  }
  return states;
}

/** Decide one finding. Re-deciding is allowed; the log remembers both. */
export function decide(review: Review, index: number, verdict: Verdict): Review {
  const log = [...review.log, { findings: [index], verdict }];
  return { ...review, log, states: replay(log, review.states.length) };
}

/**
 * Decide every still-PENDING finding of a rule in one event, so a single
 * undo reverses the whole sweep. Findings already decided are untouched —
 * "accept all ipv4" should not overwrite judgments already made.
 */
export function decideRule(
  review: Review,
  ruleIndices: number[],
  verdict: Verdict,
): Review {
  const pending = ruleIndices.filter((i) => review.states[i] === "pending");
  if (pending.length === 0) return review;
  const log = [...review.log, { findings: pending, verdict }];
  return { ...review, log, states: replay(log, review.states.length) };
}

/** Pop the last event. Repeated undo walks the whole session back. */
export function undo(review: Review): Review {
  if (review.log.length === 0) return review;
  const log = review.log.slice(0, -1);
  return { ...review, log, states: replay(log, review.states.length) };
}

export function focusNext(review: Review): Review {
  if (review.focused === null) return review;
  return {
    ...review,
    focused: Math.min(review.states.length - 1, review.focused + 1),
  };
}

export function focusPrev(review: Review): Review {
  if (review.focused === null) return review;
  return { ...review, focused: Math.max(0, review.focused - 1) };
}

export function tally(review: Review) {
  let accepted = 0,
    rejected = 0,
    pending = 0;
  for (const s of review.states) {
    if (s === "accepted") accepted++;
    else if (s === "rejected") rejected++;
    else pending++;
  }
  return { accepted, rejected, pending };
}
