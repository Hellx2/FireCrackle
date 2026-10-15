import { describe, expect, it } from "vitest";
import { thumbSlots, uniqueFrames } from "./thumbs";

const vp = (start: number, pxPerFrame: number) => ({ start, pxPerFrame });

describe("thumbSlots", () => {
  it("tiles the visible video with slots of the thumbnail width", () => {
    // 1000 frames at 0.5 px = 500 px wide, 100 px thumbnails -> 5 slots
    const s = thumbSlots(vp(0, 0.5), 500, 100, 1000);
    expect(s).toHaveLength(5);
    expect(s.map((x) => x.x)).toEqual([0, 100, 200, 300, 400]);
    expect(s.every((x) => x.w === 100)).toBe(true);
  });

  it("names the frame at the centre of each slot", () => {
    const s = thumbSlots(vp(0, 0.5), 500, 100, 1000);
    expect(s.map((x) => x.frame)).toEqual([100, 300, 500, 700, 900]);
  });

  it("anchors slots to the video, so panning moves them instead of re-cutting them", () => {
    const a = thumbSlots(vp(0, 1), 300, 100, 1000);
    const b = thumbSlots(vp(40, 1), 300, 100, 1000);
    expect(a.map((x) => x.frame)).toEqual([50, 150, 250]);
    // the same video-space slot [100,200) is now 40 px further left, with the same centre frame
    const same = b.find((x) => x.frame === 150)!;
    expect(same.x).toBe(60);
  });

  it("includes the partly visible slots at both edges", () => {
    const s = thumbSlots(vp(50, 1), 200, 100, 1000);
    expect(s.map((x) => x.frame)).toEqual([50, 150, 250]);
    expect(s[0].x).toBe(-50);
  });

  it("clips the last slot at the end of the video", () => {
    const s = thumbSlots(vp(0, 1), 1000, 100, 250);
    expect(s).toHaveLength(3);
    expect(s[2].x).toBe(200);
    expect(s[2].w).toBe(50);
    expect(s[2].frame).toBe(225); // centre of the part that exists
  });

  it("shows nothing when there are no frames or the view is past the end", () => {
    expect(thumbSlots(vp(0, 1), 500, 100, 0)).toEqual([]);
    expect(thumbSlots(vp(5000, 1), 500, 100, 1000)).toEqual([]);
  });

  it("is safe for zero or invalid sizes", () => {
    expect(thumbSlots(vp(0, 1), 0, 100, 1000)).toEqual([]);
    expect(thumbSlots(vp(0, 1), 500, 0, 1000)).toEqual([]);
    expect(thumbSlots(vp(0, 0), 500, 100, 1000)).toEqual([]);
    expect(thumbSlots(vp(0, Number.NaN), 500, 100, 1000)).toEqual([]);
  });

  it("copes with a thumbnail wider than the whole view", () => {
    const s = thumbSlots(vp(0, 1), 80, 200, 1000);
    expect(s).toHaveLength(1);
    expect(s[0].frame).toBe(100);
  });

  it("copes with a single-frame video", () => {
    const s = thumbSlots(vp(0, 50), 200, 100, 1);
    expect(s).toHaveLength(1);
    expect(s[0].frame).toBe(0);
    expect(s[0].w).toBe(50);
  });

  it("never names a frame outside the video", () => {
    for (const [start, px, total] of [[0, 0.1, 7], [3, 2, 40], [0, 33, 5]] as const) {
      for (const s of thumbSlots(vp(start, px), 600, 96, total)) {
        expect(s.frame).toBeGreaterThanOrEqual(0);
        expect(s.frame).toBeLessThanOrEqual(total - 1);
      }
    }
  });

  it("makes only the visible slots when zoomed far in", () => {
    const s = thumbSlots(vp(500, 32), 900, 96, 1000);
    expect(s.length).toBeLessThanOrEqual(11);
    expect(s.length).toBeGreaterThanOrEqual(9);
  });
});

describe("uniqueFrames", () => {
  it("keeps each thumbnail once, in order, and drops missing ones", () => {
    expect(uniqueFrames([0, 0, 30, null, 30, 60, 0])).toEqual([0, 30, 60]);
  });
  it("is empty for nothing", () => {
    expect(uniqueFrames([])).toEqual([]);
    expect(uniqueFrames([null, null])).toEqual([]);
  });
});
