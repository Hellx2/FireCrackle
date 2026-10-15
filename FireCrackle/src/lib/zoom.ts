/** Zoom and pan: the pure logic behind wheel zoom on the video and ctrl+wheel zoom on panels. */

export interface Limits {
  min: number;
  max: number;
}

/** A zoomed, panned view: screen = content * scale + (x, y), content origin at the pane's top-left. */
export interface ViewXform {
  scale: number;
  x: number;
  y: number;
}

export interface Box {
  w: number;
  h: number;
}

export const VIDEO_ZOOM: Limits = { min: 1, max: 16 };
export const PANEL_ZOOM: Limits = { min: 0.5, max: 3 };

export const IDENTITY: ViewXform = { scale: 1, x: 0, y: 0 };

export interface PanelZooms {
  picker: number;
  flags: number;
}

export const DEFAULT_ZOOMS: PanelZooms = { picker: 1, flags: 1 };
export const ZOOM_STORAGE_KEY = "firecrackle.zoom.v1";

const WHEEL_RATE = 0.0015; // per wheel-delta unit; a 100-unit notch is about 16 %
const MAX_STEP = 1.6; // one wheel event never changes the scale by more than this factor

function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}

/** Scale factor for one wheel event: up (negative deltaY) zooms in. */
export function wheelFactor(deltaY: number): number {
  if (!Number.isFinite(deltaY) || deltaY === 0) return 1;
  return clamp(Math.exp(-deltaY * WHEEL_RATE), 1 / MAX_STEP, MAX_STEP);
}

/** A panel's zoom after one wheel event. */
export function zoomPanel(scale: number, deltaY: number): number {
  const start = Number.isFinite(scale) ? scale : 1;
  return clamp(start * wheelFactor(deltaY), PANEL_ZOOM.min, PANEL_ZOOM.max);
}

/** Keep the view valid: scale in range, and no empty space past an edge of the content. */
export function clampView(v: ViewXform, box: Box): ViewXform {
  const scale = Number.isFinite(v.scale) ? clamp(v.scale, VIDEO_ZOOM.min, VIDEO_ZOOM.max) : 1;
  const w = Number.isFinite(box.w) ? Math.max(0, box.w) : 0;
  const h = Number.isFinite(box.h) ? Math.max(0, box.h) : 0;
  const x = Number.isFinite(v.x) ? v.x : 0;
  const y = Number.isFinite(v.y) ? v.y : 0;
  return {
    scale,
    x: clamp(x, w - w * scale, 0),
    y: clamp(y, h - h * scale, 0),
  };
}

/** Zoom by `factor` around the pane point (px, py): the content under it stays under it. */
export function zoomAt(v: ViewXform, px: number, py: number, factor: number, box: Box): ViewXform {
  if (!(box.w > 0) || !(box.h > 0)) return clampView(v, box);
  const f = Number.isFinite(factor) && factor > 0 ? factor : 1;
  const next = clamp(v.scale * f, VIDEO_ZOOM.min, VIDEO_ZOOM.max);
  const cx = (px - v.x) / v.scale;
  const cy = (py - v.y) / v.scale;
  return clampView({ scale: next, x: px - cx * next, y: py - cy * next }, box);
}

/** Move the content with the pointer by (dx, dy). */
export function panView(v: ViewXform, dx: number, dy: number, box: Box): ViewXform {
  return clampView({ scale: v.scale, x: v.x + dx, y: v.y + dy }, box);
}

function zoomField(raw: unknown, fallback: number): number {
  if (typeof raw !== "number" || !Number.isFinite(raw)) return fallback;
  return clamp(raw, PANEL_ZOOM.min, PANEL_ZOOM.max);
}

export function parseZooms(raw: string | null): PanelZooms {
  let obj: unknown = null;
  if (raw) {
    try {
      obj = JSON.parse(raw);
    } catch {
      obj = null;
    }
  }
  const o = obj && typeof obj === "object" && !Array.isArray(obj) ? (obj as Record<string, unknown>) : {};
  return { picker: zoomField(o.picker, DEFAULT_ZOOMS.picker), flags: zoomField(o.flags, DEFAULT_ZOOMS.flags) };
}

export function serializeZooms(z: PanelZooms): string {
  return JSON.stringify(z);
}
