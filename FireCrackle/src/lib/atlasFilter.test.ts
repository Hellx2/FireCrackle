import { expect, it } from "vitest";
import { animationPool, appearanceId, nextAppearance, appearanceEdges, edgeFilterValue, matchesEdges, matchingAppearances, sheetEdgeCounts, type AtlasEdges } from "./atlasFilter";

it("counts only visible pixels at x=0 and x=15, in each animation row", () => {
  const rgba = new Uint8ClampedArray(16 * 512 * 4);
  for (let y = 0; y < 3; y++) rgba[(y * 16) * 4 + 3] = 255;
  for (let y = 4; y < 11; y++) rgba[(y * 16 + 15) * 4 + 3] = 255;
  rgba[(16 * 16) * 4 + 3] = 1; // Any visible alpha counts, including translucent pixels.
  rgba[(17 * 16 + 7) * 4 + 3] = 255; // Interior pixel must not affect either edge.
  rgba[(20 * 16) * 4] = 255; // RGB under transparent alpha must not count.
  const rows = sheetEdgeCounts(rgba);
  expect(rows).toHaveLength(32);
  expect(rows[0]).toEqual({ left: 3, right: 7 });
  expect(rows[1]).toEqual({ left: 1, right: 0 });
  expect(rows[31]).toEqual({ left: 0, right: 0 });
  expect(() => sheetEdgeCounts(new Uint8ClampedArray(10))).toThrow();
});

it("swaps edge counts for mirrored appearances and supports independent filters", () => {
  const edges: AtlasEdges = [Array.from({ length: 32 }, () => ({ left: 3, right: 7 })), Array.from({ length: 32 }, () => ({ left: 2, right: 9 }))];
  expect(appearanceEdges(edges, 0, 0, true)).toEqual({ left: 7, right: 3 });
  expect(matchingAppearances(edges, { left: null, right: null })).toBe(128);
  expect(matchingAppearances(edges, { left: 3, right: 7 })).toBe(32);
  expect(matchingAppearances(edges, { left: 7, right: 3 })).toBe(32);
  expect(matchingAppearances(edges, { left: 3, right: 9 })).toBe(0);
  expect(matchesEdges({ left: 0, right: 16 }, { left: 0, right: null })).toBe(true);
});

it("keeps zero distinct from Any and rejects invalid counts", () => {
  expect(edgeFilterValue("0")).toBe(0);
  expect(edgeFilterValue("16")).toBe(16);
  for (const value of ["", " ", "17", "-1", "2.5", "NaN"]) expect(edgeFilterValue(value)).toBeNull();
});
it("visits all 128 appearances once before wrapping, independent of filters when disabled", () => {
  const pool=animationPool(null);
  expect(new Set(pool.map(s=>appearanceId(s.sheet,s.row,s.mirrored))).size).toBe(128);
  expect(nextAppearance(pool,pool[127])).toEqual(pool[0]);
  expect(nextAppearance(pool,pool[0],-1)).toEqual(pool[127]);
  expect(nextAppearance([],pool[0])).toBeNull();
});
it("combines mirrored edge counts with reversible per-appearance exclusions", () => {
  const edges:AtlasEdges=[Array.from({length:32},()=>({left:3,right:7})),Array.from({length:32},()=>({left:2,right:9}))];
  const excluded=[appearanceId(0,0,true)];
  const filter={left:7,right:3,excluded};
  const pool=animationPool(edges,filter);
  expect(pool).toHaveLength(31);
  expect(pool.every(s=>s.sheet===0&&s.mirrored&&s.row!==0)).toBe(true);
  expect(matchingAppearances(edges,filter)).toBe(31);
  expect(animationPool(null,filter)).toHaveLength(0);
  expect(animationPool(null,{left:null,right:null,excluded})).toHaveLength(127);
  expect(animationPool(edges,{...filter,excluded:[]})).toHaveLength(32);
});
