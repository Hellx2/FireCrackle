// Geometry of the timeline thumbnail row. Pure functions (tested in thumbs.test.ts); drawing is in Timeline.tsx.

import type { Viewport } from "./timeline";

export type ThumbState = "idle" | "running" | "ready" | "failed" | "disabled";

export interface ThumbStatus {
  state: ThumbState;
  count: number;
  error: string | null;
}

export interface ThumbSlot {
  /** Left edge on screen in pixels (negative when the slot starts left of the view). */
  x: number;
  /** Width in pixels (the last slot is cut at the end of the video). */
  w: number;
  /** The video frame at the centre of the slot: the thumbnail nearest to it is drawn. */
  frame: number;
}

/**
 * Slots for the thumbnails that are on screen. Slots are anchored to the video (slot k covers video pixels
 * [k*thumbWidth, (k+1)*thumbWidth)), so panning moves the same slots instead of cutting new ones.
 */
export function thumbSlots(vp: Viewport, width: number, thumbWidthPx: number, total: number): ThumbSlot[] {
  const px = vp.pxPerFrame;
  if (!(width > 0) || !(thumbWidthPx > 0) || !(px > 0) || !Number.isFinite(px) || !Number.isFinite(vp.start) || total <= 0) return [];
  const videoPx = total * px;
  const left = vp.start * px;
  const first = Math.max(0, Math.floor(left / thumbWidthPx));
  const slots: ThumbSlot[] = [];
  for (let k = first; ; k++) {
    const v0 = k * thumbWidthPx;
    if (v0 >= videoPx || v0 - left >= width) break;
    const v1 = Math.min(v0 + thumbWidthPx, videoPx);
    const frame = Math.min(total - 1, Math.max(0, Math.floor((v0 + v1) / 2 / px)));
    slots.push({ x: v0 - left, w: v1 - v0, frame });
  }
  return slots;
}

/** Each thumbnail frame once, in order of first use; missing ones dropped. */
export function uniqueFrames(frames: Array<number | null>): number[] {
  const seen = new Set<number>();
  const out: number[] = [];
  for (const f of frames) {
    if (f !== null && !seen.has(f)) {
      seen.add(f);
      out.push(f);
    }
  }
  return out;
}
