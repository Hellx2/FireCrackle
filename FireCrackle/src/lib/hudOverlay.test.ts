import { describe, expect, it } from "vitest";
import { hudQuads, projectHud, hudTexels, hudRowFromLabel, stepHudRow } from "./hudOverlay";

describe("first-person HUD geometry", () => {
  it("matches independently computed vanilla quad corners", () => {
    const want = [[35.707, 119.057], [150.995, 148.624], [150.995, 25.344], [35.707, 32.736]];
    hudQuads()[0].forEach((p, i) => {
      const got = projectHud(p, 100, 100, 90);
      expect(got[0]).toBeCloseTo(want[i][0], 2);
      expect(got[1]).toBeCloseTo(want[i][1], 2);
    });
  });
  it("mirrors the quads and respects viewport aspect", () => {
    const q = hudQuads();
    for (const [w, h] of [[1920, 1080], [800, 600]]) {
      const a = projectHud(q[0][0], w, h, 70);
      const b = projectHud(q[1][1], w, h, 70);
      expect(a[0] + b[0]).toBeCloseTo(w);
      expect(a[1]).toBeCloseTo(b[1]);
    }
  });
  it("reverses both UV axes exactly as the HUD renderer does", () => {
    const rgba = new Uint8ClampedArray(16 * 512 * 4);
    rgba.set([255, 100, 0, 255], 0); // top-left texel in row 0
    const pixels = hudTexels(rgba, 0, 100, 100, 90);
    expect(pixels).toHaveLength(2);
    const topLeft = pixels[0].points[0];
    const projectedTopRight = projectHud(hudQuads()[0][2], 100, 100, 90);
    expect(topLeft[0]).toBeCloseTo(projectedTopRight[0], 10);
    expect(topLeft[1]).toBeCloseTo(projectedTopRight[1], 10);
    expect(pixels[0].colour).toBe("rgb(255, 100, 0)");
    expect(pixels[0].alpha).toBe(0.9);
    expect(pixels.map((p) => p.quad)).toEqual([0, 1]);
  });
  it("reads the requested animation row and skips transparent texels", () => {
    const rgba = new Uint8ClampedArray(16 * 512 * 4);
    rgba.set([1, 2, 3, 128], 31 * 16 * 16 * 4);
    expect(hudTexels(rgba, 0, 1920, 1080, 70)).toEqual([]);
    const result = hudTexels(rgba, 31, 1920, 1080, 70);
    expect(result).toHaveLength(2);
    expect(result[0].alpha).toBeCloseTo(0.9 * 128 / 255);
  });
  it("rejects invalid geometry and assets", () => {
    const rgba = new Uint8ClampedArray(16 * 512 * 4);
    for (const [row, w, h, fov] of [[32,100,100,70], [-1,100,100,70], [0,0,100,70], [0,100,100,180], [0,100,100,NaN]]) {
      expect(() => hudTexels(rgba, row, w, h, fov)).toThrow();
    }
    expect(() => hudTexels(new Uint8ClampedArray(4), 0, 100, 100, 70)).toThrow();
  });
  it("uses sheet 1 phase when copying a label, independent of picker mirroring", () => {
    expect(hudRowFromLabel({ fire: { sheet: 1, row: 3, mirrored: false } })).toBe(3);
    expect(hudRowFromLabel({ fire: { sheet: 0, row: 31, mirrored: true } })).toBe(15);
    expect(hudRowFromLabel("unset")).toBeNull();
    expect(hudRowFromLabel("no_fire")).toBeNull();
  });
  it("wraps row stepping at both ends", () => {
    expect(stepHudRow(31, 1)).toBe(0);
    expect(stepHudRow(0, -1)).toBe(31);
  });
});
