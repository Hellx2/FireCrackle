import type { View } from "./api";
import type { UiAction } from "./actions";
import { atlasSprite, type PreviewSprite } from "./firePreview";
import { isFire, labelToCell, type Label } from "./grid";

export interface CandidateDraft { frame: number; label: Label; origin: "picked" | "typed"; digits?: string }
export interface Candidate { label: Label; origin: "picked" | "typed" | "guess" | "saved" }
export function currentCandidate(view: Pick<View, "current" | "label" | "suggestion">, draft: CandidateDraft | null): Candidate | null {
  if (draft?.frame === view.current) return draft.label === "unset" ? null : { label: draft.label, origin: draft.origin };
  if (view.label !== "unset") return { label: view.label, origin: "saved" };
  const s = view.suggestion;
  if (s && labelToCell({ fire: s })) return { label: { fire: { sheet: s.sheet, row: s.row, mirrored: s.mirrored } }, origin: "guess" };
  return null;
}
export function candidateConfirmation(candidate: Candidate | null): UiAction | null {
  if (!candidate) return null;
  if (candidate.origin === "guess") return { type: "acceptSuggestion" };
  const label = candidate.label;
  if (label === "no_fire") return { type: "noFire" };
  if (label === "skip") return { type: "skip" };
  const cell = labelToCell(label);
  return cell ? { type: "clickCell", ...cell } : null;
}
export function candidateDigit(frame: number, draft: CandidateDraft | null, digit: number, pick: { sheet: number; mirrored: boolean }): CandidateDraft {
  const pending = draft?.frame === frame && draft.digits?.length === 1 && Number(draft.digits) <= 3;
  const digits = pending ? draft!.digits + String(digit) : String(digit);
  const row = Number(digits);
  const col = (pick.mirrored ? 2 : 0) + pick.sheet;
  const sprite = atlasSprite(col, row);
  return { frame, origin: "typed", label: sprite ? { fire: sprite } : "unset", digits };
}
export function spriteText(sprite: PreviewSprite | { sheet: number; row: number; mirrored: boolean }): string {
  return `row ${sprite.row} · sheet ${sprite.sheet} · ${sprite.mirrored ? "mirrored" : "normal"}`;
}
export function candidateText(label: Label | null): string {
  return label && isFire(label) ? spriteText(label.fire) : label === "no_fire" ? "No fire" : label === "skip" ? "Skip" : "None selected";
}
