import { expect, it } from "vitest";
import { candidateConfirmation, candidateDigit, currentCandidate } from "./labelCandidate";
const view = { current: 10, label: "unset" as const, suggestion: { sheet: 1, row: 7, mirrored: false, alt_row: 8, from_frame: 1 } };
it("requires a separate confirmation action and never carries a draft into another frame", () => {
  const draft = { frame: 10, label: { fire: { sheet: 0, row: 31, mirrored: true } }, origin: "picked" as const };
  expect(candidateConfirmation(currentCandidate(view, draft))).toEqual({ type: "clickCell", col: 2, row: 31 });
  expect(candidateConfirmation(currentCandidate({ ...view, current: 11 }, draft))).toEqual({ type: "acceptSuggestion" });
  expect(currentCandidate({ ...view, suggestion: null }, null)).toBeNull();
});
it("uses the saved label before a guess, and keeps no-fire and skip reviewed labels", () => {
  expect(currentCandidate({ ...view, label: "no_fire" }, null)).toEqual({ label: "no_fire", origin: "saved" });
  expect(candidateConfirmation({ label: "no_fire", origin: "picked" })).toEqual({ type: "noFire" });
  expect(candidateConfirmation({ label: "skip", origin: "picked" })).toEqual({ type: "skip" });
});
it("typing only produces a draft, with invalid rows blocking confirmation", () => {
  const pick = { sheet: 0, mirrored: true };
  const first = candidateDigit(10, null, 3, pick);
  expect(first.label).toEqual({ fire: { ...pick, row: 3 } });
  const valid = candidateDigit(10, first, 1, pick);
  expect(candidateConfirmation(currentCandidate(view, valid))).toEqual({ type: "clickCell", col: 2, row: 31 });
  const invalid = candidateDigit(10, first, 5, pick);
  expect(candidateConfirmation(currentCandidate(view, invalid))).toBeNull();
});
