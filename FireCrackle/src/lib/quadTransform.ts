import { validQuad, type ManualQuad, type QuadPoint } from "./quadMask";

export interface BoxHandle { x: -1 | 0 | 1; y: -1 | 0 | 1 }
export const BOX_HANDLES: BoxHandle[] = [
  { x: -1, y: -1 }, { x: 0, y: -1 }, { x: 1, y: -1 }, { x: 1, y: 0 },
  { x: 1, y: 1 }, { x: 0, y: 1 }, { x: -1, y: 1 }, { x: -1, y: 0 },
];
export function quadBounds(q: ManualQuad) {
  const left = Math.min(...q.map(p => p[0])), right = Math.max(...q.map(p => p[0]));
  const top = Math.min(...q.map(p => p[1])), bottom = Math.max(...q.map(p => p[1]));
  return { left, top, right, bottom, cx: (left + right) / 2, cy: (top + bottom) / 2 };
}
export function boxPoint(q: ManualQuad, h: BoxHandle): QuadPoint {
  const b = quadBounds(q);
  return [h.x < 0 ? b.left : h.x > 0 ? b.right : b.cx, h.y < 0 ? b.top : h.y > 0 ? b.bottom : b.cy];
}

/** Resize from the starting quad, anchored at the opposite box corner/edge. */
export function resizeQuad(q: ManualQuad, h: BoxHandle, start: QuadPoint, cursor: QuadPoint, width: number, height: number): ManualQuad {
  if (start[0] === cursor[0] && start[1] === cursor[1]) return q;
  const anchor = boxPoint(q, { x: -h.x as BoxHandle["x"], y: -h.y as BoxHandle["y"] });
  const handle = boxPoint(q, h);
  const dx = (handle[0] - anchor[0]) * width, dy = (handle[1] - anchor[1]) * height;
  const px = dx + (cursor[0] - start[0]) * width, py = dy + (cursor[1] - start[1]) * height;
  let sx = 1, sy = 1;
  if (h.x && h.y) sx = sy = Math.max(0.02, (px * dx + py * dy) / (dx * dx + dy * dy));
  else if (h.x) sx = Math.max(0.02, px / dx);
  else if (h.y) sy = Math.max(0.02, py / dy);
  const next = q.map(([x, y]) => [sx === 1 ? x : anchor[0] + (x - anchor[0]) * sx, sy === 1 ? y : anchor[1] + (y - anchor[1]) * sy]) as ManualQuad;
  return validQuad(next) ? next : q;
}

/** Rotate in video pixels so non-square frames do not distort the mask. */
export function rotateQuad(q: ManualQuad, start: QuadPoint, cursor: QuadPoint, width: number, height: number): ManualQuad {
  const { cx, cy } = quadBounds(q);
  const a = [(start[0] - cx) * width, (start[1] - cy) * height];
  const b = [(cursor[0] - cx) * width, (cursor[1] - cy) * height];
  if (Math.hypot(...a) < 1e-6 || Math.hypot(...b) < 1e-6) return q;
  const angle = Math.atan2(a[0] * b[1] - a[1] * b[0], a[0] * b[0] + a[1] * b[1]);
  const c = Math.cos(angle), s = Math.sin(angle);
  const next = q.map(([x, y]) => {
    const px = (x - cx) * width, py = (y - cy) * height;
    return [cx + (px * c - py * s) / width, cy + (px * s + py * c) / height];
  }) as ManualQuad;
  return validQuad(next) ? next : q;
}

/** Closest parallelogram by equal corner corrections; keeps the center fixed. */
export function parallelizeQuad(q: ManualQuad): ManualQuad {
  const error = [0, 1].map(axis => (q[0][axis] + q[2][axis] - q[1][axis] - q[3][axis]) / 4);
  if (error.every(n => Math.abs(n) < 1e-12)) return q;
  const next = q.map(([x, y], i) => [x + (i % 2 ? 1 : -1) * error[0], y + (i % 2 ? 1 : -1) * error[1]]) as ManualQuad;
  return validQuad(next) ? next : q;
}
