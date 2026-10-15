import { expect, it } from "vitest";
import { DEFAULT_RULER, moveRuler, readRuler, rulerTicks, rulerUnit, scaleRulerUnit, type RulerPoints } from "./ruler";

it("uses only0 and1 as calibration and expands into negative and positive units", () => {
  const points: RulerPoints = [[0.4, 0.5], [0.5, 0.5]];
  const axis = rulerTicks(points, 1000, 500)!;
  expect(axis.unit).toBeCloseTo(100);
  expect(axis.ticks.map(t => t.index)).toContain(-4);
  expect(axis.ticks.map(t => t.index)).toContain(6);
  expect(axis.ticks.find(t => t.index === 1)!.x).toBeCloseTo(100);
  expect(axis.angle).toBe(0);
});
it("measures rotated unit lengths in video pixels rather than stretched normalized coordinates", () => {
  const points: RulerPoints = [[0.2, 0.2], [0.3, 0.4]];
  expect(rulerUnit(points, 2000, 1000)).toBeCloseTo(Math.sqrt(80000));
  const changed = scaleRulerUnit(points, 20, 2000, 1000);
  expect(changed[0]).toBe(points[0]);
  expect(rulerUnit(changed, 2000, 1000)).toBeCloseTo(20);
  expect(rulerTicks(changed, 2000, 1000)!.angle).toBeCloseTo(45);
});
it("moves the ruler without changing the0-to1 spacing, or edits either calibration point", () => {
  const points = DEFAULT_RULER.points;
  const moved = moveRuler(points, "move", [-2, 1]);
  expect(rulerUnit(moved, 1920, 1080)).toBeCloseTo(rulerUnit(points, 1920, 1080));
  expect(moved[0][0]).toBeLessThan(0);
  expect(moveRuler(points, 1, [0.1, 0])[0]).toEqual(points[0]);
  expect(moveRuler(points, 0, [0.1, 0])[1]).toEqual(points[1]);
  expect(moveRuler(points, 1, [points[0][0] - points[1][0], 0])).toBe(points);
});
it("bounds tick density for tiny units, huge zoom, and off-image calibration", () => {
  for (const zoom of [0.001, 1, 10000]) {
    const points: RulerPoints = [[-0.1, 0.5], [-0.0999999, 0.5]];
    const axis = rulerTicks(points, 3840, 2160, zoom)!;
    expect(axis.ticks.length).toBeLessThanOrEqual(302);
    expect(axis.stride).toBeGreaterThan(1);
  }
  expect(rulerTicks([[0, 0], [0, 0]], 1000, 500)).toBeNull();
  expect(scaleRulerUnit(DEFAULT_RULER.points, NaN, 1000, 500)).toBe(DEFAULT_RULER.points);
});
it("restores valid calibration and ignores malformed or collapsed saved state", () => {
  const state = { enabled: true, points: [[-0.5, 0], [0.5, 0.1]] };
  expect(readRuler({ getItem: () => JSON.stringify(state) })).toEqual(state);
  expect(readRuler({ getItem: () => '{"enabled":true,"points":[null,null]}' })).toEqual(DEFAULT_RULER);
  expect(readRuler({ getItem: () => '{"enabled":true,"points":[[0,0],[0,0]]}' })).toEqual(DEFAULT_RULER);
});
