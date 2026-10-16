import { describe, expect, it } from "vitest";
import type { Bucket } from "./api";
import type { ChartPoint } from "./chart";
import { bridgeStatus, findEvidenceGaps, gapKind, gapText } from "./evidenceGaps";

const FPS = 60;
const run = (from: number, to: number) => Array.from({ length: to - from + 1 }, (_, i) => from + i);
const bucket = (b: Partial<Bucket>): Bucket => ({ frames: 100, unset: 0, fire: 0, no_fire: 0, skip: 0, flag: null, ...b });
const pt = (frame: number, phase: number, step = 0): ChartPoint => ({ frame, pts: frame / FPS, step, phase });

describe("findEvidenceGaps", () => {
  it("finds the stretch between two evidence runs", () => {
    // evidence on frames 0..99 and 700..799: 600 frames (10 s) with none between
    const gaps = findEvidenceGaps([...run(0, 99), ...run(700, 799)], 800, FPS, 2);
    expect(gaps).toHaveLength(1);
    expect(gaps[0]).toMatchObject({ first: 100, last: 699, edge: "interior" });
    expect(gaps[0].seconds).toBeCloseTo((700 - 99) / FPS, 6);
  });

  it("reports nothing for continuous evidence", () => {
    expect(findEvidenceGaps(run(0, 599), 600, FPS, 2)).toEqual([]);
  });

  it("ignores a gap shorter than the minimum, and keeps one exactly at it", () => {
    const short = findEvidenceGaps([...run(0, 9), ...run(60, 70)], 71, FPS, 2); // 50 frames, under 1 s
    expect(short).toEqual([]);
    const exact = findEvidenceGaps([0, 120], 121, FPS, 2); // 120 frames = exactly 2 s between observations
    expect(exact).toHaveLength(1);
  });

  it("marks stretches before the first and after the last evidence as edges", () => {
    const gaps = findEvidenceGaps(run(600, 700), 1400, FPS, 2);
    expect(gaps.map((g) => g.edge)).toEqual(["start", "end"]);
    expect(gaps[0]).toMatchObject({ first: 0, last: 599 });
    expect(gaps[1]).toMatchObject({ first: 701, last: 1399 });
  });

  it("is one whole-video gap when there is no evidence at all", () => {
    const gaps = findEvidenceGaps([], 1200, FPS, 2);
    expect(gaps).toEqual([expect.objectContaining({ first: 0, last: 1199, edge: "whole" })]);
  });

  it("is empty for an empty or unusable video", () => {
    expect(findEvidenceGaps([], 0, FPS, 2)).toEqual([]);
    expect(findEvidenceGaps([1, 2], 100, 0, 2)).toEqual([]);
    expect(findEvidenceGaps([1, 2], 100, NaN, 2)).toEqual([]);
  });

  it("copes with unsorted, duplicate, out-of-range and non-finite frames", () => {
    const gaps = findEvidenceGaps([700, 0, 0, 50, NaN, -5, 5000, 700, Infinity], 900, FPS, 2);
    expect(gaps.map((g) => [g.first, g.last, g.edge])).toEqual([
      [51, 699, "interior"],
      [701, 899, "end"],
    ]);
  });

  it("a single evidence frame leaves a start and an end gap when both are long enough", () => {
    const gaps = findEvidenceGaps([600], 1200, FPS, 2);
    expect(gaps.map((g) => g.edge)).toEqual(["start", "end"]);
  });

  it("does not mutate its input", () => {
    const frames = [700, 0];
    findEvidenceGaps(frames, 800, FPS, 2);
    expect(frames).toEqual([700, 0]);
  });
});

describe("gapKind", () => {
  it("is no_fire when most frames are labelled no fire", () => {
    expect(gapKind(bucket({ no_fire: 80, unset: 20 }))).toBe("no_fire");
  });
  it("is unlabelled when mostly unset (or skipped)", () => {
    expect(gapKind(bucket({ unset: 90, no_fire: 10 }))).toBe("unlabelled");
    expect(gapKind(bucket({ skip: 100 }))).toBe("unlabelled");
  });
  it("is other_source when fire labels exist but did not count as evidence", () => {
    expect(gapKind(bucket({ fire: 70, unset: 30 }))).toBe("other_source");
  });
  it("an empty or missing bucket is unlabelled, never an error", () => {
    expect(gapKind(bucket({ frames: 0 }))).toBe("unlabelled");
    expect(gapKind(null)).toBe("unlabelled");
  });
});

describe("bridgeStatus", () => {
  const gap = { first: 100, last: 699, frames: 600, seconds: 10, edge: "interior" as const };

  it("compares the clock just before and just after an interior gap", () => {
    const b = bridgeStatus(gap, [pt(99, 10), pt(700, 10.4)]);
    expect(b).not.toBeNull();
    expect(b!.deltaTicks).toBeCloseTo(0.4, 6);
    expect(b!.agrees).toBe(true);
  });
  it("reports a disagreement beyond the tolerance, with its sign", () => {
    const b = bridgeStatus(gap, [pt(99, 10), pt(700, 7)])!;
    expect(b.deltaTicks).toBeCloseTo(-3, 6);
    expect(b.agrees).toBe(false);
  });
  it("wraps around the 32-tick seam: 31.8 vs 0.2 is a 0.4 tick difference", () => {
    const b = bridgeStatus(gap, [pt(99, 31.8), pt(700, 0.2)])!;
    expect(b.deltaTicks).toBeCloseTo(0.4, 6);
    expect(b.agrees).toBe(true);
  });
  it("has nothing to say for an edge gap or when a side has no point", () => {
    expect(bridgeStatus({ ...gap, edge: "start" }, [pt(99, 1), pt(700, 1)])).toBeNull();
    expect(bridgeStatus({ ...gap, edge: "whole" }, [])).toBeNull();
    expect(bridgeStatus(gap, [pt(99, 1)])).toBeNull();
    expect(bridgeStatus(gap, [pt(700, 1)])).toBeNull();
  });
  it("uses the nearest kept points even when the chart was thinned", () => {
    const b = bridgeStatus(gap, [pt(40, 5), pt(95, 10), pt(705, 10.1), pt(900, 3)])!;
    expect(b.deltaTicks).toBeCloseTo(0.1, 6);
  });
});

describe("gapText", () => {
  const gap = { first: 100, last: 699, frames: 600, seconds: 10, edge: "interior" as const };

  it("always says lag, a freeze or a cut cannot be ruled out, and never claims proof", () => {
    for (const kind of ["no_fire", "unlabelled", "other_source"] as const) {
      const text = gapText(gap, kind, null);
      expect(text).toMatch(/cannot be ruled out/i);
      expect(text).not.toMatch(/proof|proves|proven|confirmed/i);
    }
  });
  it("names the kind of stretch and its length", () => {
    expect(gapText(gap, "no_fire", null)).toMatch(/labelled no fire/i);
    expect(gapText(gap, "unlabelled", null)).toMatch(/unlabelled/i);
    expect(gapText(gap, "other_source", null)).toMatch(/excluded source|time guess/i);
    expect(gapText(gap, "no_fire", null)).toMatch(/10(\.0)? s/);
  });
  it("says a matching clock does not exclude a whole 1.6 s cycle", () => {
    const text = gapText(gap, "no_fire", { deltaTicks: 0.4, agrees: true });
    expect(text).toMatch(/agrees/i);
    expect(text).toMatch(/whole.*cycle|1\.6 s/i);
  });
  it("reports a mismatch as a possible lag, cut or mislabel, with the signed delta", () => {
    const text = gapText(gap, "no_fire", { deltaTicks: -3, agrees: false });
    expect(text).toMatch(/-3(\.0)? ticks/);
    expect(text).toMatch(/lag|cut|mislabel/i);
  });
  it("describes an edge gap as outside the checked stretch", () => {
    const text = gapText({ ...gap, edge: "start" }, "unlabelled", null);
    expect(text).toMatch(/before the first|start/i);
  });
});
