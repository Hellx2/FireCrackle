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

/** Repair invalid numeric values without limiting zoom or off-frame navigation. */
export function clampView(v: ViewXform, _box: Box): ViewXform {
  const scale = Number.isFinite(v.scale) && v.scale > 0 ? v.scale : 1;
  const x = Number.isFinite(v.x) ? v.x : 0;
  const y = Number.isFinite(v.y) ? v.y : 0;
  return {
    scale,
    x,
    y,
  };
}

/** Zoom by `factor` around the pane point (px, py): the content under it stays under it. */
export function zoomAt(v: ViewXform, px: number, py: number, factor: number, box: Box): ViewXform {
  if (!(box.w > 0) || !(box.h > 0)) return clampView(v, box);
  const start = clampView(v, box);
  const f = Number.isFinite(factor) && factor > 0 ? factor : 1;
  // Only numeric representation has limits: prevent zero/Infinity at underflow/overflow.
  const next = Math.min(Number.MAX_VALUE, Math.max(Number.MIN_VALUE, start.scale * f));
  const ratio = next / start.scale;
  return clampView({ scale: next, x: px - (px - start.x) * ratio, y: py - (py - start.y) * ratio }, box);
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
