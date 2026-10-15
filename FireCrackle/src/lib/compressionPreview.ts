import type { ManualQuad } from "./quadMask";

export interface CompressionSettings { height: number; quality: number }
export interface CompressionTexel { points: Array<[number, number]>; colour: string; alpha: number }

/** Delivery coordinates only: CSS size, devicePixelRatio, zoom and pan never enter here. */
export function compressionGeometry(width: number, height: number, targetHeight: number, quad: ManualQuad) {
  // The decoder may supply a reduced preview. Its aspect ratio locates the quad,
  // but only the selected delivery resolution determines the fire's raster size.
  const h = Math.max(1, Math.round(targetHeight));
  const w = Math.max(1, Math.round(width * h / height));
  return { width:w, height:h,
    quadWidth:(Math.max(...quad.map(p=>p[0]))-Math.min(...quad.map(p=>p[0])))*w,
    quadHeight:(Math.max(...quad.map(p=>p[1]))-Math.min(...quad.map(p=>p[1])))*h };
}

/** SVG is rasterized at delivery size BEFORE any lossy encoding or editor magnification. */
export function compressionSvg(texels: CompressionTexel[], width: number, height: number) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" overflow="hidden"><g shape-rendering="crispEdges">${texels.map(p =>
    `<polygon points="${p.points.map(([x,y])=>`${x*width},${y*height}`).join(" ")}" fill="${p.colour}" fill-opacity="${p.alpha}"/>`).join("")}</g></svg>`;
}
