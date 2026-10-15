export interface EdgeCounts { left: number; right: number }
export interface EdgeFilter { left: number | null; right: number | null }
export type AtlasEdges = [EdgeCounts[], EdgeCounts[]];

/** A fire pixel is a nontransparent texel; internal columns are deliberately ignored. */
export function sheetEdgeCounts(rgba: Uint8ClampedArray): EdgeCounts[] {
  if (rgba.length !== 16 * 512 * 4) throw new Error("Expected a 16×512 fire sprite sheet");
  return Array.from({ length: 32 }, (_, row) => {
    let left = 0, right = 0;
    for (let y = row * 16; y < row * 16 + 16; y++) {
      if (rgba[(y * 16) * 4 + 3] > 0) left++;
      if (rgba[(y * 16 + 15) * 4 + 3] > 0) right++;
    }
    return { left, right };
  });
}
export function appearanceEdges(edges: AtlasEdges, sheet: 0 | 1, row: number, mirrored: boolean): EdgeCounts {
  const counts = edges[sheet][row];
  return mirrored ? { left: counts.right, right: counts.left } : counts;
}
export function matchesEdges(counts: EdgeCounts, filter: EdgeFilter): boolean {
  return (filter.left === null || filter.left === counts.left) && (filter.right === null || filter.right === counts.right);
}
export function edgeFilterValue(value: string): number | null {
  if (!value.trim()) return null;
  const count = Number(value);
  return Number.isInteger(count) && count >= 0 && count <= 16 ? count : null;
}
export function matchingAppearances(edges: AtlasEdges, filter: EdgeFilter): number {
  let matches = 0;
  for (const sheet of [0, 1] as const) for (let row = 0; row < 32; row++) for (const mirrored of [false, true]) {
    if (matchesEdges(appearanceEdges(edges, sheet, row, mirrored), filter)) matches++;
  }
  return matches;
}
