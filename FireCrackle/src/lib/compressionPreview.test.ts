import { describe, expect, it } from "vitest";
import { compressionGeometry, compressionSvg } from "./compressionPreview";
import type { ManualQuad } from "./quadMask";

const quad: ManualQuad = [[0.4, 0.4], [0.42, 0.4], [0.42, 0.44], [0.4, 0.44]];
describe("compression at delivery pixels", () => {
  it("maps a 2% wide quad to 38.4 pixels at 1920, independently of editor zoom", () => {
    const g = compressionGeometry(2560, 1440, 1080, quad);
    expect(g.width).toBe(1920);
    expect(g.height).toBe(1080);
    expect(g.quadWidth).toBeCloseTo(38.4);
    expect(g.quadHeight).toBeCloseTo(43.2);
    expect(compressionGeometry(2560, 1440, 540, quad).quadWidth).toBeCloseTo(19.2);
  });
  it("honors selected delivery resolution even when the decoded preview is smaller", () => {
    expect(compressionGeometry(640, 360, 1080, quad)).toMatchObject({ width:1920, height:1080 });
    const from720 = compressionGeometry(1280,720,1080,quad);
    expect(from720).toEqual(compressionGeometry(2560,1440,1080,quad));
    expect(from720.quadWidth).toBeCloseTo(38.4);
  });
  it("preserves aspect ratio and permits off-frame quads", () => {
    const outside: ManualQuad = [[-0.1,0], [0.1,0], [0.1,1], [-0.1,1]];
    expect(compressionGeometry(1920,1080,720,outside).quadWidth).toBeCloseTo(256);
    expect(compressionGeometry(1080,1920,720,quad)).toMatchObject({ width:405,height:720 });
  });
  it("rasterizes projected texels in delivery coordinates, with clipping and no editor transform", () => {
    const svg = compressionSvg([{points:quad,colour:"rgb(255, 0, 0)",alpha:1}],1920,1080);
    expect(svg).toContain('width="1920" height="1080"');
    expect(svg).toContain('points="768,432 806.4,432 806.4,475.2 768,475.2"');
    expect(svg).toContain('shape-rendering="crispEdges"');
    expect(svg).not.toContain("transform=");
  });
});
