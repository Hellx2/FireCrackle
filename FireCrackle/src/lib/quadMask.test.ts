import { describe, expect, it } from "vitest";
import { DEFAULT_QUAD, manualTexels, moveQuadCorner, projectQuad, validQuad, type ManualQuad } from "./quadMask";

describe("manual fire quad", () => {
  it("maps all four sprite corners exactly onto an asymmetric perspective quad", () => {
    const quad: ManualQuad = [[0.2, 0.1], [0.9, 0.25], [0.75, 0.9], [0.1, 0.7]];
    for (const [i, [u, v]] of [[0, 0], [1, 0], [1, 1], [0, 1]].entries()) {
      const point = projectQuad(quad, u, v);
      expect(point[0]).toBeCloseTo(quad[i][0], 10);
      expect(point[1]).toBeCloseTo(quad[i][1], 10);
    }
    // A projective transform preserves straight lines across the sprite.
    const line = [0, 0.25, 0.5, 0.75, 1].map((u) => projectQuad(quad, u, 0.5));
    const [a, b] = [line[0], line[4]];
    for (const p of line) expect((p[0] - a[0]) * (b[1] - a[1]) - (p[1] - a[1]) * (b[0] - a[0])).toBeCloseTo(0, 10);
  });

  it("allows handles outside the image and prevents folded or collapsed masks", () => {
    expect(validQuad(DEFAULT_QUAD)).toBe(true);
    const moved = moveQuadCorner(DEFAULT_QUAD, 0, [-2, -1]);
    expect(moved[0]).toEqual([-2, -1]);
    expect(moved.slice(1)).toEqual(DEFAULT_QUAD.slice(1));
    expect(moveQuadCorner(DEFAULT_QUAD, 0, [0.8, 0.9])).toBe(DEFAULT_QUAD);
    expect(moveQuadCorner(DEFAULT_QUAD, 0, DEFAULT_QUAD[1])).toBe(DEFAULT_QUAD);
    expect(moveQuadCorner(DEFAULT_QUAD, 5, [0, 0])).toBe(DEFAULT_QUAD);
  });

  it("supports quads partly or entirely outside the video without clipping geometry", () => {
    const outside: ManualQuad = [[-2,-2],[-1,-2],[-1,-1],[-2,-1]];
    expect(validQuad(outside)).toBe(true);
    expect(projectQuad(outside,0,0)).toEqual([-2,-2]);
    expect(moveQuadCorner(DEFAULT_QUAD,2,[1.4,1.6])[2]).toEqual([1.4,1.6]);
    expect(moveQuadCorner(DEFAULT_QUAD,0,[NaN,0])).toBe(DEFAULT_QUAD);
  });

  it("keeps source row, transparency and manual mirroring separate from geometry", () => {
    const rgba = new Uint8ClampedArray(16 * 512 * 4);
    rgba.set([255, 128, 10, 255], 31 * 16 * 16 * 4);
    expect(manualTexels(rgba, 0, false, DEFAULT_QUAD)).toEqual([]);
    const normal = manualTexels(rgba, 31, false, DEFAULT_QUAD);
    const mirrored = manualTexels(rgba, 31, true, DEFAULT_QUAD);
    expect(normal).toHaveLength(1);
    expect(mirrored).toHaveLength(1);
    expect(normal[0].points[0]).toEqual(DEFAULT_QUAD[0]);
    expect(mirrored[0].points[1]).toEqual(DEFAULT_QUAD[1]);
    expect(normal[0].colour).toBe("rgb(255, 128, 10)");
    expect(normal[0].alpha).toBe(1);
  });
});
