import { describe, expect, it } from "vitest";
import type { Flag } from "./api";
import type { ChartPoint } from "./chart";
import { compareFrames, describePoint, spriteRowOf } from "./flagCompare";

const FPS = 60;
const pt = (frame: number, phase = 0, step = frame % 32): ChartPoint => ({ frame, pts: frame / FPS, step, phase });
const flag = (kind: Flag["kind"], frame: number, end_frame = frame): Flag => ({ kind, severity: "hard", frame, end_frame, phase_delta: 3 });
const points = [pt(10), pt(20), pt(30), pt(40), pt(50), pt(60)];

describe("compareFrames", () => {
  it("a persistent shift: the last point before it, and the first at or after it", () => {
    const c = compareFrames(points, flag("phase_shift", 35));
    expect(c.before?.frame).toBe(30);
    expect(c.after?.frame).toBe(40);
    expect(c.during).toBeNull();
  });

  it("a shift that starts exactly on a point puts that point after the change", () => {
    const c = compareFrames(points, flag("phase_shift", 30));
    expect(c.before?.frame).toBe(20);
    expect(c.after?.frame).toBe(30);
  });

  it("a flag that returns: before it, the point inside it, and the first point after it ends", () => {
    const c = compareFrames(points, flag("phase_excursion", 25, 45));
    expect(c.before?.frame).toBe(20);
    expect(c.during?.frame).toBe(30);
    expect(c.after?.frame).toBe(50);
  });

  it("an odd frame is handled like a flag that returns", () => {
    const c = compareFrames(points, flag("phase_outlier", 30, 30));
    expect(c.before?.frame).toBe(20);
    expect(c.during?.frame).toBe(30);
    expect(c.after?.frame).toBe(40);
  });

  it("has no before when the flag is at the start, and no after when it is at the end", () => {
    expect(compareFrames(points, flag("phase_shift", 5)).before).toBeNull();
    expect(compareFrames(points, flag("phase_shift", 5)).after?.frame).toBe(10);
    expect(compareFrames(points, flag("phase_excursion", 55, 70)).after).toBeNull();
    expect(compareFrames(points, flag("phase_shift", 99)).after).toBeNull();
  });

  it("finds the nearest kept point when the chart was thinned and the flag frame is missing", () => {
    const thin = [pt(10), pt(100), pt(200)];
    const c = compareFrames(thin, flag("phase_shift", 150));
    expect(c.before?.frame).toBe(100);
    expect(c.after?.frame).toBe(200);
  });

  it("returns all nulls for no points and never throws", () => {
    expect(compareFrames([], flag("phase_shift", 5))).toEqual({ before: null, during: null, after: null });
  });
});

describe("spriteRowOf", () => {
  it("is the animation index on sheet 1 (row == step)", () => {
    expect(spriteRowOf(0)).toBe(0);
    expect(spriteRowOf(31)).toBe(31);
  });
  it("wraps and repairs odd input", () => {
    expect(spriteRowOf(32)).toBe(0);
    expect(spriteRowOf(-1)).toBe(31);
    expect(spriteRowOf(7.6)).toBe(8);
    expect(spriteRowOf(NaN)).toBe(0);
  });
});

describe("describePoint", () => {
  it("names the frame, time, animation index and timing offset", () => {
    const text = describePoint(pt(120, 4, 9), 4, FPS);
    expect(text).toMatch(/frame 120/);
    expect(text).toContain("00:00:02|00");
    expect(text).toMatch(/index 9/);
    expect(text).toMatch(/\+?0(\.0+)? ticks/);
  });
  it("shows a signed timing offset against the reference, wrapping at the seam", () => {
    expect(describePoint(pt(1, 0.2), 31.8, FPS)).toMatch(/\+0\.4 ticks/);
    expect(describePoint(pt(1, 31.8), 0.2, FPS)).toMatch(/-0\.4 ticks/);
  });
  it("says there is nothing to show for a missing point", () => {
    expect(describePoint(null, 0, FPS)).toMatch(/no fire evidence/i);
  });
});
