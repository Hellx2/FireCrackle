import { describe, expect, it } from "vitest";
import { DEFAULT_QUAD, validQuad, type ManualQuad } from "./quadMask";
import { parallelizeQuad, resizeQuad, rotateQuad } from "./quadTransform";

const q: ManualQuad = [[0.1, 0.2], [0.7, 0.1], [0.8, 0.7], [0.2, 0.8]];
describe("quad box transforms", () => {
  it("scales every relative distance equally and fixes the opposite corner", () => {
    const resized = resizeQuad(DEFAULT_QUAD, { x: 1, y: 1 }, [0.72, 0.82], [0.92, 1.12], 1920, 1080);
    expect(resized[0]).toEqual(DEFAULT_QUAD[0]);
    for (let i = 0; i < 4; i++) for (let j = 0; j < i; j++) {
      const distance = (p: ManualQuad) => Math.hypot((p[i][0] - p[j][0]) * 1920, (p[i][1] - p[j][1]) * 1080);
      expect(distance(resized) / distance(DEFAULT_QUAD)).toBeCloseTo(1.5);
    }
    expect(resizeQuad(q, { x: -1, y: -1 }, [0, 0], [0, 0], 1920, 1080)).toEqual(q);
  });
  it("edge resizing changes only one axis, including off-image positions", () => {
    const changed = resizeQuad(q, { x: 1, y: 0 }, [0.8, 0.5], [1.5, 3], 1920, 1080);
    changed.forEach((p, i) => { expect(p[1]).toBe(q[i][1]); expect(p[0]).toBeCloseTo(0.1 + (q[i][0] - 0.1) * 2); });
    const vertical = resizeQuad(q, { x: 0, y: -1 }, [0.5, 0.1], [8, -0.6], 1920, 1080);
    vertical.forEach((p, i) => { expect(p[0]).toBe(q[i][0]); expect(p[1]).toBeCloseTo(0.8 + (q[i][1] - 0.8) * 2); });
    expect(validQuad(changed)).toBe(true);
  });
  it("rotation preserves all pixel distances on widescreen video", () => {
    const rotated = rotateQuad(q, [0.45, 0.0], [0.9, 0.45], 1920, 1080);
    for (let i = 0; i < 4; i++) for (let j = 0; j < i; j++) {
      const length = (p: ManualQuad) => Math.hypot((p[i][0] - p[j][0]) * 1920, (p[i][1] - p[j][1]) * 1080);
      expect(length(rotated)).toBeCloseTo(length(q));
    }
    expect(validQuad(rotated)).toBe(true);
  });
  it("does not flip or collapse when a resize crosses its anchor", () => {
    expect(validQuad(resizeQuad(q, { x: 1, y: 1 }, [0.8, 0.8], [-4, -4], 1920, 1080))).toBe(true);
    expect(resizeQuad(q, { x: 1, y: 1 }, [0, 0], [NaN, 0], 1920, 1080)).toBe(q);
  });
  it("parallelizes opposite edges while preserving the center", () => {
    const input: ManualQuad = [[0.1, 0.1], [0.8, 0.2], [0.7, 0.9], [0.2, 0.7]];
    const result = parallelizeQuad(input);
    for (let axis = 0; axis < 2; axis++) {
      expect(result[1][axis] - result[0][axis]).toBeCloseTo(result[2][axis] - result[3][axis]);
      expect(result[3][axis] - result[0][axis]).toBeCloseTo(result[2][axis] - result[1][axis]);
      expect(result.reduce((sum, p) => sum + p[axis], 0)).toBeCloseTo(input.reduce((sum, p) => sum + p[axis], 0));
    }
    expect(parallelizeQuad(q)).toEqual(q);
    expect(validQuad(result)).toBe(true);
  });
});
