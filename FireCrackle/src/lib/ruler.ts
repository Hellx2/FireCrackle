import type { QuadPoint } from "./quadMask";
export type RulerPoints = [QuadPoint, QuadPoint];
export interface RulerState { enabled: boolean; points: RulerPoints }
export const DEFAULT_RULER: RulerState = { enabled: false, points: [[0.35, 0.5], [0.4, 0.5]] };
export const RULER_KEY = "firecrackle.ruler.v1";
export function validRuler(points: unknown): points is RulerPoints {
  return Array.isArray(points) && points.length === 2 && points.every(p => Array.isArray(p) && p.length === 2 && p.every(n => typeof n === "number" && Number.isFinite(n))) && Math.hypot(points[1][0] - points[0][0], points[1][1] - points[0][1]) > 1e-8;
}
export function readRuler(storage?: Pick<Storage, "getItem">): RulerState {
  try {
    const saved = JSON.parse((storage ?? localStorage).getItem(RULER_KEY) ?? "null");
    if (saved && typeof saved.enabled === "boolean" && validRuler(saved.points)) return saved;
  } catch { /* Storage is optional. */ }
  return DEFAULT_RULER;
}
export function writeRuler(state: RulerState, storage?: Pick<Storage, "setItem">): void {
  try { (storage ?? localStorage).setItem(RULER_KEY, JSON.stringify(state)); } catch { /* Storage is optional. */ }
}
export function rulerUnit(points: RulerPoints, width: number, height: number): number {
  return Math.hypot((points[1][0] - points[0][0]) * width, (points[1][1] - points[0][1]) * height);
}
export function scaleRulerUnit(points: RulerPoints, pixels: number, width: number, height: number): RulerPoints {
  const current = rulerUnit(points, width, height);
  if (!Number.isFinite(pixels) || pixels <= 0 || !Number.isFinite(current) || current <= 0) return points;
  const ratio = pixels / current;
  const next: RulerPoints = [points[0], [points[0][0] + (points[1][0] - points[0][0]) * ratio, points[0][1] + (points[1][1] - points[0][1]) * ratio]];
  return validRuler(next) ? next : points;
}
export function moveRuler(points: RulerPoints, handle: 0 | 1 | "move", delta: QuadPoint): RulerPoints {
  const next = points.map(([x, y], i) => handle === "move" || handle === i ? [x + delta[0], y + delta[1]] : [x, y]) as RulerPoints;
  return validRuler(next) ? next : points;
}
function niceStep(value: number): number {
  if (value <= 1) return 1;
  const power = 10 ** Math.floor(Math.log10(value));
  return [1, 2, 5, 10].find(n => n * power >= value)! * power;
}
/** The 0→1 vector defines a unit; the visible axis extends across the video in both directions. */
export function rulerTicks(points: RulerPoints, width: number, height: number, screenScale = 1) {
  const unit = rulerUnit(points, width, height);
  if (!validRuler(points) || ![width, height, unit, screenScale].every(Number.isFinite) || width <= 0 || height <= 0 || unit <= 0 || screenScale <= 0) return null;
  const x = points[0][0] * width, y = points[0][1] * height;
  const ux = (points[1][0] - points[0][0]) * width / unit, uy = (points[1][1] - points[0][1]) * height / unit;
  const projections = [[0, 0], [width, 0], [width, height], [0, height]].map(([px, py]) => (px - x) * ux + (py - y) * uy);
  const min = Math.min(0, ...projections), max = Math.max(unit, ...projections);
  if (![min, max, x, y].every(Number.isFinite)) return null;
  const stride = niceStep(Math.max((max - min) / unit / 300, 8 / (unit * screenScale)));
  const labelStride = stride * niceStep(45 / (unit * screenScale * stride));
  const ticks: Array<{ index: number; x: number; label: boolean }> = [];
  for (let index = Math.ceil(min / unit / stride) * stride; index <= max / unit && ticks.length <= 301; index += stride) {
    ticks.push({ index: Object.is(index, -0) ? 0 : index, x: index * unit, label: index % labelStride === 0 });
  }
  return { x, y, angle: Math.atan2(uy, ux) * 180 / Math.PI, min, max, unit, stride, ticks };
}
