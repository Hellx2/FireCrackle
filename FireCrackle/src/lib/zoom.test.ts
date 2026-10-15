import { describe, expect, it } from "vitest";
import {
  IDENTITY,
  PANEL_ZOOM,
  clampView,
  panView,
  parseZooms,
  serializeZooms,
  wheelFactor,
  zoomAt,
  zoomPanel,
  DEFAULT_ZOOMS,
} from "./zoom";

const BOX = { w: 1000, h: 500 };

describe("wheelFactor", () => {
  it("scrolling up (negative deltaY) zooms in, down zooms out", () => {
    expect(wheelFactor(-100)).toBeGreaterThan(1);
    expect(wheelFactor(100)).toBeLessThan(1);
  });
  it("no scroll is no change", () => {
    expect(wheelFactor(0)).toBe(1);
  });
  it("opposite scrolls cancel out", () => {
    expect(wheelFactor(-120) * wheelFactor(120)).toBeCloseTo(1, 10);
  });
  it("one notch is a modest step and a huge delta is capped", () => {
    expect(wheelFactor(-100)).toBeLessThan(1.5);
    expect(wheelFactor(-1e6)).toBeLessThan(3);
    expect(wheelFactor(1e6)).toBeGreaterThan(0.3);
  });
  it("NaN is no change", () => {
    expect(wheelFactor(NaN)).toBe(1);
  });
});

describe("zoomPanel", () => {
  it("multiplies the scale and stays within the panel limits", () => {
    expect(zoomPanel(1, -100)).toBeGreaterThan(1);
    expect(zoomPanel(PANEL_ZOOM.max, -100)).toBe(PANEL_ZOOM.max);
    expect(zoomPanel(PANEL_ZOOM.min, 100)).toBe(PANEL_ZOOM.min);
  });
  it("repairs an out-of-range or NaN starting scale", () => {
    expect(zoomPanel(NaN, 0)).toBe(1);
    expect(zoomPanel(99, 0)).toBe(PANEL_ZOOM.max);
  });
});

describe("zoomAt (video)", () => {
  it("zooming in at the centre keeps the centre fixed", () => {
    const v = zoomAt(IDENTITY, 500, 250, 2, BOX);
    expect(v.scale).toBe(2);
    // the centre of the content is still at the centre of the pane
    expect(v.x).toBe(-500);
    expect(v.y).toBe(-250);
  });
  it("keeps the point under the cursor fixed (the defining property)", () => {
    const start = { scale: 2, x: -300, y: -100 };
    const px = 700;
    const py = 200;
    const cx = (px - start.x) / start.scale;
    const cy = (py - start.y) / start.scale;
    const v = zoomAt(start, px, py, 1.5, BOX);
    expect(cx * v.scale + v.x).toBeCloseTo(px, 6);
    expect(cy * v.scale + v.y).toBeCloseTo(py, 6);
  });
  it("zooms out below fit while keeping the cursor anchored", () => {
    const v = zoomAt({ scale: 1.2, x: -50, y: -20 }, 300, 100, 0.1, BOX);
    expect(v.scale).toBeCloseTo(0.12);
    expect(v.x).toBeCloseTo(265);
    expect(v.y).toBeCloseTo(88);
  });
  it("has no application zoom ceiling", () => {
    const v = zoomAt(IDENTITY, 0, 0, 1e6, BOX);
    expect(v.scale).toBe(1e6);
    expect(zoomAt(v, 0, 0, 2, BOX).scale).toBe(2e6);
  });
  it("has no application zoom-out floor", () => {
    let v = IDENTITY;
    for (let i = 0; i < 200; i++) v = zoomAt(v, 500, 250, 0.5, BOX);
    expect(v.scale).toBe(2 ** -200);
    expect(v.x).toBeCloseTo(500);
    expect(v.y).toBeCloseTo(250);
  });
  it("zooming in at the corner never exposes empty space", () => {
    const v = zoomAt(IDENTITY, 0, 0, 4, BOX);
    expect(v.x).toBeLessThanOrEqual(0);
    expect(v.x).toBeGreaterThanOrEqual(BOX.w - BOX.w * v.scale);
    const w = zoomAt(IDENTITY, BOX.w, BOX.h, 4, BOX);
    expect(w.x).toBeCloseTo(BOX.w - BOX.w * 4, 6);
    expect(w.y).toBeCloseTo(BOX.h - BOX.h * 4, 6);
  });
  it("an empty box or a NaN factor changes nothing and never produces NaN", () => {
    expect(zoomAt(IDENTITY, 0, 0, 2, { w: 0, h: 0 })).toEqual(IDENTITY);
    const v = zoomAt({ scale: 2, x: -10, y: -10 }, 5, 5, NaN, BOX);
    expect(Number.isNaN(v.scale + v.x + v.y)).toBe(false);
  });
});

describe("clampView / panView", () => {
  it("pans freely at fit and zoomed out", () => {
    expect(panView(IDENTITY, 80, -40, BOX)).toEqual({scale:1,x:80,y:-40});
    expect(panView({scale:0.25,x:0,y:0}, -80, 90, BOX)).toEqual({scale:0.25,x:-80,y:90});
  });
  it("panning moves the content with the pointer", () => {
    const v = panView({ scale: 2, x: -400, y: -100 }, 50, 20, BOX);
    expect(v).toEqual({ scale: 2, x: -350, y: -80 });
  });
  it("can pan past any video edge", () => {
    const v = panView({ scale: 2, x: -400, y: -100 }, 9999, 9999, BOX);
    expect(v.x).toBe(9599);
    expect(v.y).toBe(9899);
    const w = panView({ scale: 2, x: -400, y: -100 }, -9999, -9999, BOX);
    expect(w.x).toBe(-10399);
    expect(w.y).toBe(-10099);
  });
  it("preserves off-frame positioning after a resize", () => {
    const v = clampView({ scale: 2, x: -1800, y: -900 }, { w: 400, h: 200 });
    expect(v.x).toBe(-1800);
    expect(v.y).toBe(-900);
  });
  it("preserves any finite positive scale and repairs invalid values", () => {
    expect(clampView({ scale: 0.2, x: 5, y: 5 }, BOX)).toEqual({ scale:0.2,x:5,y:5 });
    expect(clampView({ scale: 999, x: 0, y: 0 }, BOX).scale).toBe(999);
    expect(clampView({ scale: NaN, x: Infinity, y: NaN }, BOX)).toEqual(IDENTITY);
    expect(clampView({ scale: 0, x: 0, y: 0 }, BOX)).toEqual(IDENTITY);
  });
  it("keeps finite coordinates at numeric overflow or underflow", () => {
    for (const scale of [Number.MAX_VALUE, Number.MIN_VALUE]) {
      const v = zoomAt({scale,x:0,y:0}, 200, 100, scale > 1 ? 2 : 0.5, BOX);
      expect(v.scale).toBeGreaterThan(0);
      expect([v.scale,v.x,v.y].every(Number.isFinite)).toBe(true);
    }
  });
});

describe("persisting panel zooms", () => {
  it("round-trips", () => {
    const z = { picker: 1.5, flags: 0.75 };
    expect(parseZooms(serializeZooms(z))).toEqual(z);
  });
  it("defaults when nothing or garbage is stored", () => {
    expect(parseZooms(null)).toEqual(DEFAULT_ZOOMS);
    expect(parseZooms("nope")).toEqual(DEFAULT_ZOOMS);
    expect(parseZooms("[1,2]")).toEqual(DEFAULT_ZOOMS);
  });
  it("clamps and repairs fields individually", () => {
    const z = parseZooms(JSON.stringify({ picker: 99, flags: "big" }));
    expect(z.picker).toBe(PANEL_ZOOM.max);
    expect(z.flags).toBe(DEFAULT_ZOOMS.flags);
  });
});
