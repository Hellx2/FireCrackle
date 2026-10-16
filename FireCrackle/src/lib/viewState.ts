import { clampViewport, fitViewport, type Viewport } from "./timeline";
import { IDENTITY, type ViewXform } from "./zoom";

type Store = Pick<Storage, "getItem" | "setItem">;
function key(video: string, kind: string) { return `firecrackle.view.v1:${encodeURIComponent(video)}:${kind}`; }
function read(video: string | undefined, kind: string, storage?: Pick<Store, "getItem">): any {
  if (!video) return null;
  try { return JSON.parse((storage ?? localStorage).getItem(key(video, kind)) ?? "null"); } catch { return null; }
}
function write(video: string | undefined, kind: string, value: unknown, storage?: Pick<Store, "setItem">) {
  if (!video) return;
  try { (storage ?? localStorage).setItem(key(video, kind), JSON.stringify(value)); } catch { /* Optional UI memory. */ }
}
export function readVideoView(video?: string, storage?: Pick<Store, "getItem">): ViewXform {
  const saved = read(video, "video", storage);
  return saved && [saved.scale, saved.x, saved.y].every(n => typeof n === "number" && Number.isFinite(n)) && saved.scale > 0
    ? { scale: saved.scale, x: saved.x, y: saved.y, ...(typeof saved.angle === "number" && Number.isFinite(saved.angle) ? { angle: saved.angle % 360 } : {}) } : IDENTITY;
}
export function writeVideoView(video: string | undefined, value: ViewXform, storage?: Pick<Store, "setItem">) { write(video, "video", value, storage); }
export function readTimelineView(video: string | undefined, kind: "timeline" | "phase", total: number, width: number, storage?: Pick<Store, "getItem">): Viewport {
  const saved = read(video, kind, storage);
  if (!saved || saved.total !== total || ![saved.start, saved.span].every(n => typeof n === "number" && Number.isFinite(n)) || saved.span <= 0) return fitViewport(total, width);
  const pxPerFrame = width / saved.span;
  return Number.isFinite(pxPerFrame) && pxPerFrame > 0 ? clampViewport({ start: saved.start, pxPerFrame }, width, total) : fitViewport(total, width);
}
export function writeTimelineView(video: string | undefined, kind: "timeline" | "phase", total: number, width: number, value: Viewport, storage?: Pick<Store, "setItem">) {
  if (value.pxPerFrame > 0 && width > 0) write(video, kind, { total, start: value.start, span: width / value.pxPerFrame }, storage);
}
