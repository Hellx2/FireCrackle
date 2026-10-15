import { describe, expect, it } from "vitest";
import {
  flagSpans,
  hitFlag,
  nearestPoint,
  phaseToY,
  relativePhase,
  rowToY,
  timeToX,
  xToTime,
  type ChartPoint,
} from "./chart";

const P = (frame: number, pts: number, phase = 0, step = 0): ChartPoint => ({ frame, pts, step, phase });

describe("relativePhase", () => {
  it("is zero at the reference", () => {
    expect(relativePhase(5, 5)).toBe(0);
  });
  it("is the signed shortest offset on the 32-tick circle", () => {
    expect(relativePhase(7, 5)).toBe(2);
    expect(relativePhase(3, 5)).toBe(-2);
  });
  it("wraps across the 0/32 seam", () => {
    expect(relativePhase(0.5, 31.5)).toBeCloseTo(1, 9);
    expect(relativePhase(31.5, 0.5)).toBeCloseTo(-1, 9);
  });
  it("puts exactly half a circle at +16, never -16", () => {
    expect(relativePhase(21, 5)).toBe(16);
    expect(relativePhase(5, 21)).toBe(16);
  });
  it("stays inside (-16, 16] for any input, including negatives and large values", () => {
    for (const [a, b] of [[-40, 3], [100, -7], [31.999, 0], [0, 31.999], [64, 0]]) {
      const r = relativePhase(a, b);
      expect(r).toBeGreaterThan(-16);
      expect(r).toBeLessThanOrEqual(16);
    }
  });
});

describe("scales", () => {
  it("maps time to x and back", () => {
    expect(timeToX(5, 0, 10, 200)).toBe(100);
    expect(xToTime(100, 0, 10, 200)).toBe(5);
    expect(xToTime(timeToX(3.3, 1, 9, 640), 1, 9, 640)).toBeCloseTo(3.3, 9);
  });
  it("puts the middle of the chart at x when there is no time span", () => {
    expect(timeToX(4, 4, 4, 200)).toBe(100);
    expect(xToTime(37, 4, 4, 200)).toBe(4);
  });
  it("is not NaN for a zero width", () => {
    expect(Number.isFinite(timeToX(1, 0, 10, 0))).toBe(true);
    expect(Number.isFinite(xToTime(0, 0, 10, 0))).toBe(true);
  });
  it("draws a positive phase offset above the middle and a negative one below", () => {
    expect(phaseToY(0, 200)).toBe(100);
    expect(phaseToY(8, 200)).toBeLessThan(100);
    expect(phaseToY(-8, 200)).toBeGreaterThan(100);
    expect(phaseToY(16, 200)).toBe(0);
    expect(phaseToY(-16, 200)).toBe(200);
  });
  it("draws row 0 at the bottom and row 31 at the top", () => {
    expect(rowToY(0, 320)).toBe(320);
    expect(rowToY(31, 320)).toBe(0);
    expect(rowToY(16, 320)).toBeLessThan(rowToY(15, 320));
  });
});

describe("nearestPoint", () => {
  const pts = [P(0, 0), P(10, 0.5), P(20, 1.0), P(40, 2.0)];
  it("finds the closest point in time", () => {
    expect(nearestPoint(pts, 0.4)?.frame).toBe(10);
    expect(nearestPoint(pts, 1.6)?.frame).toBe(40);
  });
  it("clamps before the first and after the last", () => {
    expect(nearestPoint(pts, -5)?.frame).toBe(0);
    expect(nearestPoint(pts, 99)?.frame).toBe(40);
  });
  it("returns null for an empty chart", () => {
    expect(nearestPoint([], 1)).toBeNull();
  });
  it("breaks an exact tie toward the earlier point", () => {
    expect(nearestPoint([P(0, 0), P(10, 1)], 0.5)?.frame).toBe(0);
  });
});

describe("flag spans", () => {
  const timeOf = (f: number) => f / 20;
  const flags = [
    { frame: 100, end_frame: 100, kind: "phase_shift", severity: "soft", phase_delta: 3 },
    { frame: 200, end_frame: 260, kind: "phase_excursion", severity: "hard", phase_delta: -12 },
  ] as const;

  it("converts frames to pixel spans over the visible time range", () => {
    const s = flagSpans([...flags], timeOf, 0, 20, 400);
    expect(s[0].x0).toBeCloseTo(100, 6); // 5 s of 20 s over 400 px
    expect(s[1].x0).toBeCloseTo(200, 6);
    expect(s[1].x1).toBeCloseTo(260, 6);
  });
  it("gives a one-frame flag a non-zero width so it can be seen and hit", () => {
    const s = flagSpans([...flags], timeOf, 0, 20, 400, 6);
    expect(s[0].x1 - s[0].x0).toBeGreaterThanOrEqual(6);
  });
  it("keeps the order and the original index", () => {
    const s = flagSpans([...flags], timeOf, 0, 20, 400);
    expect(s.map((x) => x.index)).toEqual([0, 1]);
  });
  it("is empty without flags", () => {
    expect(flagSpans([], timeOf, 0, 20, 400)).toEqual([]);
  });
});

describe("hitFlag", () => {
  const spans = [
    { index: 0, x0: 100, x1: 106 },
    { index: 1, x0: 200, x1: 260 },
    { index: 2, x0: 240, x1: 246 },
  ];
  it("finds the flag under the cursor", () => {
    expect(hitFlag(spans, 103)).toBe(0);
    expect(hitFlag(spans, 220)).toBe(1);
  });
  it("misses empty space", () => {
    expect(hitFlag(spans, 150)).toBeNull();
  });
  it("lets the cursor be a few pixels off a thin flag", () => {
    expect(hitFlag(spans, 98, 4)).toBe(0);
    expect(hitFlag(spans, 98, 0)).toBeNull();
  });
  it("prefers the narrower flag where two overlap", () => {
    expect(hitFlag(spans, 243)).toBe(2);
  });
  it("is null for no spans", () => {
    expect(hitFlag([], 10)).toBeNull();
  });
});
