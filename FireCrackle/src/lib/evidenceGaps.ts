// Stretches of the video with no fire evidence for the phase check: nothing in them can be checked against
// the animation clock, so a lag stall, a freeze or a cut inside one cannot be ruled out. Pure logic only.

import type { Bucket } from "./api";
import { relativePhase, type ChartPoint } from "./chart";

export interface EvidenceGap {
  /** First and last frame (inclusive) with no evidence. */
  first: number;
  last: number;
  frames: number;
  /** Time between the evidence on either side (or the video edge): how long the clock was not observed. */
  seconds: number;
  edge: "start" | "end" | "interior" | "whole";
}

export type GapKind = "no_fire" | "unlabelled" | "other_source";

export interface Bridge {
  /** Clock offset across the gap in ticks, after minus before, in (-16, 16]. */
  deltaTicks: number;
  agrees: boolean;
}

/** A clock difference within this many ticks counts as agreeing (the checker's normal spread). */
export const BRIDGE_TOLERANCE_TICKS = 2;
/** One full cycle of the fire animation: 32 ticks at 20 Hz. */
const CYCLE_SECONDS = 1.6;

/** Gaps of at least `minSeconds` between frames that have evidence. */
export function findEvidenceGaps(evidenceFrames: number[], totalFrames: number, fps: number, minSeconds: number): EvidenceGap[] {
  if (!(totalFrames > 0) || !(fps > 0) || !Number.isFinite(fps)) return [];
  const frames = Array.from(new Set(evidenceFrames.filter((f) => Number.isFinite(f) && f >= 0 && f < totalFrames).map(Math.floor))).sort(
    (a, b) => a - b,
  );
  const gaps: EvidenceGap[] = [];
  const add = (first: number, last: number, seconds: number, edge: EvidenceGap["edge"]) => {
    if (last >= first && seconds >= minSeconds) gaps.push({ first, last, frames: last - first + 1, seconds, edge });
  };
  if (frames.length === 0) {
    add(0, totalFrames - 1, totalFrames / fps, "whole");
    return gaps;
  }
  add(0, frames[0] - 1, frames[0] / fps, "start");
  for (let i = 1; i < frames.length; i++) add(frames[i - 1] + 1, frames[i] - 1, (frames[i] - frames[i - 1]) / fps, "interior");
  const lastFrame = frames[frames.length - 1];
  add(lastFrame + 1, totalFrames - 1, (totalFrames - 1 - lastFrame) / fps, "end");
  return gaps;
}

/** What the frames of a gap are: labelled no fire, labelled fire by a source that does not count, or unlabelled. */
export function gapKind(b: Bucket | null): GapKind {
  if (!b || !(b.frames > 0)) return "unlabelled";
  if (b.no_fire * 2 >= b.frames) return "no_fire";
  if (b.fire * 2 >= b.frames) return "other_source";
  return "unlabelled";
}

/** The clock on either side of an interior gap, from the nearest chart points; null when it cannot be compared. */
export function bridgeStatus(gap: EvidenceGap, points: ChartPoint[], tolerance = BRIDGE_TOLERANCE_TICKS): Bridge | null {
  if (gap.edge !== "interior") return null;
  let before: ChartPoint | null = null;
  let after: ChartPoint | null = null;
  for (const p of points) {
    if (p.frame < gap.first) {
      if (!before || p.frame > before.frame) before = p;
    } else if (p.frame > gap.last) {
      if (!after || p.frame < after.frame) after = p;
    }
  }
  if (!before || !after) return null;
  const deltaTicks = relativePhase(after.phase, before.phase);
  return { deltaTicks, agrees: Math.abs(deltaTicks) <= tolerance };
}

const KIND_TEXT: Record<GapKind, string> = {
  no_fire: "Labelled no fire.",
  unlabelled: "Unlabelled.",
  other_source: "Only labels from a source excluded here (for example time guesses).",
};

const signed = (v: number) => `${v > 0 ? "+" : ""}${v.toFixed(1)}`;

/** The wording shown for a gap. It never claims the video is clean or edited: it says what cannot be told. */
export function gapText(gap: EvidenceGap, kind: GapKind, bridge: Bridge | null): string {
  const length = `${gap.seconds.toFixed(1)} s`;
  const where =
    gap.edge === "start" ? "before the first fire evidence" : gap.edge === "end" ? "after the last fire evidence" : gap.edge === "whole" ? "in the whole video" : "between fire evidence";
  const parts = [
    `No fire evidence for ${length}, ${where}. ${KIND_TEXT[kind]}`,
    "Lag, a freeze or a cut inside this stretch cannot be ruled out.",
  ];
  if (bridge) {
    parts.push(
      bridge.agrees
        ? `The clock agrees at both ends (Δ ${signed(bridge.deltaTicks)} ticks); that does not rule out a whole ${CYCLE_SECONDS} s cycle being lost.`
        : `The clock differs across it by Δ ${signed(bridge.deltaTicks)} ticks: a lag stall, a cut or a mislabel.`,
    );
  }
  return parts.join(" ");
}
