import { describe, expect, it } from "vitest";
import {
  decide,
  decideRule,
  emptyReview,
  focusNext,
  focusPrev,
  tally,
  undo,
} from "./review";

describe("emptyReview", () => {
  it("starts every finding pending with the first focused", () => {
    const review = emptyReview(3);
    expect(review.states).toEqual(["pending", "pending", "pending"]);
    expect(review.log).toEqual([]);
    expect(review.focused).toBe(0);
  });

  it("has nothing to focus when there are no findings", () => {
    expect(emptyReview(0).focused).toBeNull();
  });
});

describe("decide", () => {
  it("records one finding's verdict", () => {
    const review = decide(emptyReview(3), 1, "accepted");
    expect(review.states).toEqual(["pending", "accepted", "pending"]);
    expect(review.log).toEqual([{ findings: [1], verdict: "accepted" }]);
  });

  it("lets a finding be decided again, keeping both events", () => {
    const review = decide(decide(emptyReview(2), 0, "accepted"), 0, "rejected");
    expect(review.states).toEqual(["rejected", "pending"]);
    expect(review.log).toHaveLength(2);
  });

  it("does not change the review it was given", () => {
    const before = emptyReview(2);
    decide(before, 0, "accepted");
    expect(before.states).toEqual(["pending", "pending"]);
    expect(before.log).toEqual([]);
  });
});

describe("decideRule", () => {
  it("decides only the rule's pending findings, in one event", () => {
    const start = decide(emptyReview(4), 1, "rejected");
    const review = decideRule(start, [0, 1, 2], "accepted");
    expect(review.states).toEqual(["accepted", "rejected", "accepted", "pending"]);
    expect(review.log[review.log.length - 1]).toEqual({ findings: [0, 2], verdict: "accepted" });
  });

  it("returns the same review when nothing is pending", () => {
    const start = decide(emptyReview(1), 0, "accepted");
    expect(decideRule(start, [0], "rejected")).toBe(start);
  });
});

describe("undo", () => {
  it("reverses the last decision", () => {
    const review = undo(decide(decide(emptyReview(2), 0, "accepted"), 1, "rejected"));
    expect(review.states).toEqual(["accepted", "pending"]);
  });

  it("reverses a whole rule sweep at once", () => {
    const review = undo(decideRule(emptyReview(3), [0, 1, 2], "accepted"));
    expect(review.states).toEqual(["pending", "pending", "pending"]);
    expect(review.log).toEqual([]);
  });

  it("restores the earlier verdict after a re-decision", () => {
    const review = undo(decide(decide(emptyReview(1), 0, "accepted"), 0, "rejected"));
    expect(review.states).toEqual(["accepted"]);
  });

  it("returns the same review when there is nothing to undo", () => {
    const start = emptyReview(2);
    expect(undo(start)).toBe(start);
  });
});

describe("focus", () => {
  it("moves within the findings and stops at both ends", () => {
    const start = emptyReview(2);
    expect(focusPrev(start).focused).toBe(0);
    const last = focusNext(focusNext(start));
    expect(last.focused).toBe(1);
    expect(focusPrev(last).focused).toBe(0);
  });

  it("does nothing when there are no findings", () => {
    const start = emptyReview(0);
    expect(focusNext(start)).toBe(start);
    expect(focusPrev(start)).toBe(start);
  });
});

describe("tally", () => {
  it("counts each state", () => {
    const review = decide(decide(emptyReview(4), 0, "accepted"), 1, "rejected");
    expect(tally(review)).toEqual({ accepted: 1, rejected: 1, pending: 2 });
  });
});
