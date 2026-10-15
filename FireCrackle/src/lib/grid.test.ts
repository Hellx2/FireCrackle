import { describe, expect, it } from "vitest";
import {
  COLUMNS,
  ROWS,
  cellToLabel,
  columnHeader,
  isFire,
  labelToCell,
  labelsEqual,
  suggestionCells,
  type Label,
} from "./grid";

describe("the grid layout", () => {
  it("has 4 columns and 32 rows, in the order of the reference picture", () => {
    expect(COLUMNS).toHaveLength(4);
    expect(ROWS).toBe(32);
    expect(COLUMNS.map((c) => `${c.mirrored ? "M" : "N"}${c.sheet}`)).toEqual(["N0", "N1", "M0", "M1"]);
  });

  it("names the columns like the picture headers", () => {
    expect(columnHeader(0)).toEqual({ group: "Normal", sheet: "0" });
    expect(columnHeader(1)).toEqual({ group: "Normal", sheet: "1" });
    expect(columnHeader(2)).toEqual({ group: "Mirrored", sheet: "0" });
    expect(columnHeader(3)).toEqual({ group: "Mirrored", sheet: "1" });
    expect(columnHeader(4)).toBeNull();
    expect(columnHeader(-1)).toBeNull();
  });
});

describe("cell <-> label", () => {
  it("maps a cell to the Rust JSON label shape", () => {
    expect(cellToLabel(1, 5)).toEqual({ fire: { sheet: 1, row: 5, mirrored: false } });
    expect(cellToLabel(2, 31)).toEqual({ fire: { sheet: 0, row: 31, mirrored: true } });
    expect(cellToLabel(0, 0)).toEqual({ fire: { sheet: 0, row: 0, mirrored: false } });
    expect(cellToLabel(3, 7)).toEqual({ fire: { sheet: 1, row: 7, mirrored: true } });
  });

  it("round-trips all 128 cells", () => {
    for (let col = 0; col < 4; col++) {
      for (let row = 0; row < 32; row++) {
        const label = cellToLabel(col, row);
        expect(label).not.toBeNull();
        expect(labelToCell(label as Label)).toEqual({ col, row });
      }
    }
  });

  it("gives null outside the grid and for non-integer cells", () => {
    for (const [c, r] of [[-1, 0], [4, 0], [0, -1], [0, 32], [0.5, 1], [1, 2.5], [NaN, 0], [0, Infinity]]) {
      expect(cellToLabel(c, r), `${c},${r}`).toBeNull();
    }
  });

  it("gives no cell for labels that are not a valid fire label", () => {
    expect(labelToCell("unset")).toBeNull();
    expect(labelToCell("no_fire")).toBeNull();
    expect(labelToCell("skip")).toBeNull();
    expect(labelToCell({ fire: { sheet: 2, row: 0, mirrored: false } })).toBeNull();
    expect(labelToCell({ fire: { sheet: 0, row: 32, mirrored: false } })).toBeNull();
    expect(labelToCell({ fire: { sheet: 0, row: -1, mirrored: true } })).toBeNull();
    expect(labelToCell({ fire: { sheet: 0, row: 1.5, mirrored: true } })).toBeNull();
  });
});

describe("label helpers", () => {
  it("recognises fire labels", () => {
    expect(isFire({ fire: { sheet: 0, row: 1, mirrored: false } })).toBe(true);
    expect(isFire("skip")).toBe(false);
    expect(isFire("unset")).toBe(false);
  });

  it("compares labels by value", () => {
    const a: Label = { fire: { sheet: 1, row: 5, mirrored: false } };
    expect(labelsEqual(a, { fire: { sheet: 1, row: 5, mirrored: false } })).toBe(true);
    expect(labelsEqual(a, { fire: { sheet: 1, row: 5, mirrored: true } })).toBe(false);
    expect(labelsEqual(a, { fire: { sheet: 0, row: 5, mirrored: false } })).toBe(false);
    expect(labelsEqual(a, { fire: { sheet: 1, row: 6, mirrored: false } })).toBe(false);
    expect(labelsEqual("skip", "skip")).toBe(true);
    expect(labelsEqual("skip", "no_fire")).toBe(false);
    expect(labelsEqual(a, "unset")).toBe(false);
    expect(labelsEqual("unset", a)).toBe(false);
  });
});

describe("suggestion highlighting", () => {
  it("highlights the likely cell and the alternative in the same column", () => {
    const s = suggestionCells({ sheet: 1, mirrored: true, row: 8, alt_row: 7 });
    expect(s.primary).toEqual({ col: 3, row: 8 });
    expect(s.alt).toEqual({ col: 3, row: 7 });
  });

  it("has no alternative when the advance is certain", () => {
    const s = suggestionCells({ sheet: 0, mirrored: false, row: 31, alt_row: null });
    expect(s.primary).toEqual({ col: 0, row: 31 });
    expect(s.alt).toBeNull();
  });

  it("copes with an invalid suggestion without throwing", () => {
    const s = suggestionCells({ sheet: 5, mirrored: false, row: 3, alt_row: 4 });
    expect(s.primary).toBeNull();
    expect(s.alt).toBeNull();
    const t = suggestionCells({ sheet: 1, mirrored: false, row: 3, alt_row: 99 });
    expect(t.primary).toEqual({ col: 1, row: 3 });
    expect(t.alt).toBeNull();
  });
});
