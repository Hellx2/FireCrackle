// Geometry for the Phase tab's charts. Pure functions only (tested in chart.test.ts); the canvas code lives in
// components/PhaseChart.tsx.

import type { Flag } from "./api";

const CIRCLE = 32;
const HALF = CIRCLE / 2;

export interface ChartPoint {
  frame: number;
  pts: number;
  /** The displayed sprite step (sheet 1 row == step). */
  step: number;
  /** `(tick_rate * pts - step) mod 32`, in [0, 32). */
  phase: number;
}

export interface ChartRun {
  first_frame: number;
  last_frame: number;
  points: number;
  center: number;
  width: number;
}

export interface PhaseChart {
  points: ChartPoint[];
  runs: ChartRun[];
  flags: Flag[];
  tick_rate: number;
  window_ticks: number;
}

/** Signed shortest offset from `reference` to `phase` on the 32-tick circle, in (-16, 16]. */
export function relativePhase(phase: number, reference: number): number {
  const r = (((phase - reference + HALF) % CIRCLE) + CIRCLE) % CIRCLE - HALF;
  return r === -HALF ? HALF : r;
}

/** x pixel of time `t` when [t0, t1] spans `width` pixels (the middle when there is no time span). */
export function timeToX(t: number, t0: number, t1: number, width: number): number {
  if (!(t1 > t0)) return width / 2;
  return ((t - t0) / (t1 - t0)) * width;
}

export function xToTime(x: number, t0: number, t1: number, width: number): number {
  if (!(t1 > t0) || !(width > 0)) return t0;
  return t0 + (x / width) * (t1 - t0);
}

/** y pixel of a phase offset in [-16, 16] ticks: 0 in the middle, +16 at the top. */
export function phaseToY(rel: number, height: number): number {
  return height / 2 - (rel / HALF) * (height / 2);
}

/** y pixel of a sprite step 0..31: row 0 at the bottom, row 31 at the top. */
export function rowToY(step: number, height: number): number {
  return height - (step / (CIRCLE - 1)) * height;
}

/** The point closest in time to `t` (the earlier one on an exact tie); null for an empty series. */
export function nearestPoint(points: ChartPoint[], t: number): ChartPoint | null {
  if (points.length === 0) return null;
  let lo = 0;
  let hi = points.length - 1;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (points[mid].pts < t) lo = mid + 1;
    else hi = mid;
  }
  if (lo === 0) return points[0];
  const before = points[lo - 1];
  const after = points[lo];
  return t - before.pts <= after.pts - t ? before : after;
}

export interface FlagSpan {
  /** Index into the flag list. */
  index: number;
  x0: number;
  x1: number;
}

/** Pixel spans of the flags; a span is at least `minWidth` wide so a one-frame flag can be seen and clicked. */
export function flagSpans(
  flags: Flag[],
  timeOf: (frame: number) => number,
  t0: number,
  t1: number,
  width: number,
  minWidth = 4,
): FlagSpan[] {
  return flags.map((f, index) => {
    const a = timeToX(timeOf(f.frame), t0, t1, width);
    const b = timeToX(timeOf(f.end_frame), t0, t1, width);
    const x0 = Math.min(a, b);
    const x1 = Math.max(a, b, x0 + minWidth);
    return { index, x0, x1 };
  });
}

/** The flag under `x` (within `slop` pixels); where several overlap, the narrowest wins. */
export function hitFlag(spans: FlagSpan[], x: number, slop = 3): number | null {
  let best: FlagSpan | null = null;
  for (const s of spans) {
    if (x >= s.x0 - slop && x <= s.x1 + slop) {
      if (!best || s.x1 - s.x0 < best.x1 - best.x0) best = s;
    }
  }
  return best ? best.index : null;
}
