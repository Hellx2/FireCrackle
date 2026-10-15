/** Pane sizes: the pure logic behind the draggable splitters. */

export interface Limits {
  min: number;
  max: number;
}

export interface Layout {
  /** width of the right-hand picker column, px */
  sideWidth: number;
  /** height of the video frame, px */
  videoHeight: number;
  /** height of the flag list, px */
  flagsHeight: number;
}

export const SIDE_LIMITS: Limits = { min: 170, max: 720 };
export const VIDEO_LIMITS: Limits = { min: 120, max: 1400 };
export const FLAGS_LIMITS: Limits = { min: 40, max: 600 };

export const DEFAULT_LAYOUT: Layout = { sideWidth: 210, videoHeight: 380, flagsHeight: 150 };

export const PICKER_CELL_MIN = 24;
export const PICKER_CELL_MAX = 96;

export const LAYOUT_STORAGE_KEY = "firecrackle.layout.v1";

const STEP = 16;
const BIG_STEP = 64;

// picker chrome around the 4 sprite columns: row gutter, padding, border, scrollbar, cell margins
const PICKER_CHROME_PX = 48;
const PICKER_CELL_MARGIN_PX = 2;

/** Whole pixels inside [min, max]; if the range is inverted (tiny window) min wins. */
export function clampSize(value: number, min: number, max: number): number {
  const hi = Math.max(min, max);
  if (Number.isNaN(value)) return min;
  return Math.min(hi, Math.max(min, Math.round(value)));
}

/** The size after dragging `delta` px; `sign` -1 for a splitter whose pane lies on its far side. */
export function dragSize(start: number, delta: number, sign: 1 | -1, min: number, max: number): number {
  return clampSize(start + sign * delta, min, max);
}

/** Biggest a pane may get: the container minus what the other pane needs, never above `hardMax`. */
export function upperBound(container: number, otherMin: number, hardMax: number): number {
  return Math.max(0, Math.min(hardMax, container - otherMin));
}

/** New size for a key press on a focused splitter, or null when the key is not one of ours. */
export function keyboardSize(
  current: number,
  key: string,
  big: boolean,
  min: number,
  max: number,
  sign: 1 | -1 = 1,
): number | null {
  const step = big ? BIG_STEP : STEP;
  switch (key) {
    case "ArrowRight":
    case "ArrowDown":
      return clampSize(current + sign * step, min, max);
    case "ArrowLeft":
    case "ArrowUp":
      return clampSize(current - sign * step, min, max);
    case "Home":
      return clampSize(min, min, max);
    case "End":
      return clampSize(max, min, max);
    default:
      return null;
  }
}

function field(raw: unknown, limits: Limits, fallback: number): number {
  if (typeof raw !== "number" || Number.isNaN(raw)) return fallback;
  return clampSize(raw, limits.min, limits.max);
}

/** Read a stored layout; anything unusable falls back to the default, field by field. */
export function parseLayout(raw: string | null): Layout {
  let obj: unknown = null;
  if (raw) {
    try {
      obj = JSON.parse(raw);
    } catch {
      obj = null;
    }
  }
  const o = obj && typeof obj === "object" && !Array.isArray(obj) ? (obj as Record<string, unknown>) : {};
  return {
    sideWidth: field(o.sideWidth, SIDE_LIMITS, DEFAULT_LAYOUT.sideWidth),
    videoHeight: field(o.videoHeight, VIDEO_LIMITS, DEFAULT_LAYOUT.videoHeight),
    flagsHeight: field(o.flagsHeight, FLAGS_LIMITS, DEFAULT_LAYOUT.flagsHeight),
  };
}

export function serializeLayout(layout: Layout): string {
  return JSON.stringify(layout);
}

/** Sprite size in the picker for a given column width: the grid fills the panel, so wider = bigger. */
export function pickerCellSize(sideWidth: number): number {
  const column = (sideWidth - PICKER_CHROME_PX) / 4 - PICKER_CELL_MARGIN_PX;
  return clampSize(column, PICKER_CELL_MIN, PICKER_CELL_MAX);
}
