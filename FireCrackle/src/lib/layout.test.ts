import { describe, expect, it } from "vitest";
import {
  DEFAULT_LAYOUT,
  FLAGS_LIMITS,
  PICKER_CELL_MAX,
  PICKER_CELL_MIN,
  SIDE_LIMITS,
  VIDEO_LIMITS,
  clampSize,
  dragSize,
  keyboardSize,
  parseLayout,
  pickerCellSize,
  serializeLayout,
  upperBound,
} from "./layout";

describe("clampSize", () => {
  it("keeps a value inside the range and leaves inner values alone", () => {
    expect(clampSize(100, 50, 200)).toBe(100);
    expect(clampSize(10, 50, 200)).toBe(50);
    expect(clampSize(999, 50, 200)).toBe(200);
  });
  it("rounds to whole pixels", () => {
    expect(clampSize(100.6, 50, 200)).toBe(101);
  });
  it("lets min win when the range is inverted (a tiny window)", () => {
    expect(clampSize(100, 150, 120)).toBe(150);
  });
  it("turns NaN / Infinity into something usable", () => {
    expect(clampSize(NaN, 50, 200)).toBe(50);
    expect(clampSize(Infinity, 50, 200)).toBe(200);
    expect(clampSize(-Infinity, 50, 200)).toBe(50);
  });
});

describe("dragSize", () => {
  it("grows when dragging in the positive direction of a normal splitter", () => {
    expect(dragSize(300, 40, 1, 100, 600)).toBe(340);
  });
  it("an inverted splitter (panel on the far side) shrinks for a positive drag", () => {
    expect(dragSize(300, 40, -1, 100, 600)).toBe(260);
  });
  it("stops at the limits however far the pointer goes", () => {
    expect(dragSize(300, 5000, 1, 100, 600)).toBe(600);
    expect(dragSize(300, -5000, 1, 100, 600)).toBe(100);
  });
  it("a zero drag changes nothing", () => {
    expect(dragSize(300, 0, 1, 100, 600)).toBe(300);
  });
});

describe("upperBound", () => {
  it("is what is left of the container after the other pane's minimum", () => {
    expect(upperBound(1000, 320, 700)).toBe(680);
  });
  it("never exceeds the hard max", () => {
    expect(upperBound(5000, 320, 700)).toBe(700);
  });
  it("is never negative in a collapsed container", () => {
    expect(upperBound(0, 320, 700)).toBe(0);
    expect(upperBound(100, 320, 700)).toBe(0);
  });
});

describe("keyboardSize", () => {
  it("arrows nudge by a step, shift by a big step", () => {
    expect(keyboardSize(300, "ArrowRight", false, 100, 600)).toBe(316);
    expect(keyboardSize(300, "ArrowLeft", false, 100, 600)).toBe(284);
    expect(keyboardSize(300, "ArrowRight", true, 100, 600)).toBe(364);
  });
  it("up/down work too, and Home / End jump to the limits", () => {
    expect(keyboardSize(300, "ArrowDown", false, 100, 600)).toBe(316);
    expect(keyboardSize(300, "ArrowUp", false, 100, 600)).toBe(284);
    expect(keyboardSize(300, "Home", false, 100, 600)).toBe(100);
    expect(keyboardSize(300, "End", false, 100, 600)).toBe(600);
  });
  it("respects the limits and ignores other keys", () => {
    expect(keyboardSize(598, "ArrowRight", true, 100, 600)).toBe(600);
    expect(keyboardSize(300, "a", false, 100, 600)).toBeNull();
  });
  it("honours an inverted splitter", () => {
    expect(keyboardSize(300, "ArrowRight", false, 100, 600, -1)).toBe(284);
  });
});

describe("the default layout", () => {
  it("sits inside every limit", () => {
    for (const [v, l] of [
      [DEFAULT_LAYOUT.sideWidth, SIDE_LIMITS],
      [DEFAULT_LAYOUT.videoHeight, VIDEO_LIMITS],
      [DEFAULT_LAYOUT.flagsHeight, FLAGS_LIMITS],
    ] as const) {
      expect(v).toBeGreaterThanOrEqual(l.min);
      expect(v).toBeLessThanOrEqual(l.max);
    }
  });
});

describe("persisting the layout", () => {
  it("round-trips", () => {
    const l = { sideWidth: 333, videoHeight: 411, flagsHeight: 99 };
    expect(parseLayout(serializeLayout(l))).toEqual(l);
  });
  it("falls back to the defaults when nothing is stored", () => {
    expect(parseLayout(null)).toEqual(DEFAULT_LAYOUT);
  });
  it("falls back to the defaults for garbage", () => {
    expect(parseLayout("not json")).toEqual(DEFAULT_LAYOUT);
    expect(parseLayout("42")).toEqual(DEFAULT_LAYOUT);
    expect(parseLayout("null")).toEqual(DEFAULT_LAYOUT);
    expect(parseLayout("[]")).toEqual(DEFAULT_LAYOUT);
  });
  it("repairs fields one by one: bad types default, out-of-range values clamp", () => {
    const got = parseLayout(JSON.stringify({ sideWidth: "wide", videoHeight: 1e9, flagsHeight: -5 }));
    expect(got.sideWidth).toBe(DEFAULT_LAYOUT.sideWidth);
    expect(got.videoHeight).toBe(VIDEO_LIMITS.max);
    expect(got.flagsHeight).toBe(FLAGS_LIMITS.min);
  });
  it("ignores unknown fields and fills missing ones", () => {
    const got = parseLayout(JSON.stringify({ sideWidth: 300, extra: 1 }));
    expect(got).toEqual({ ...DEFAULT_LAYOUT, sideWidth: 300 });
  });
});

describe("pickerCellSize", () => {
  it("is bigger than the old 22 px sprite at the default side width", () => {
    expect(pickerCellSize(DEFAULT_LAYOUT.sideWidth)).toBeGreaterThanOrEqual(32);
  });
  it("grows with the panel and shrinks with it", () => {
    expect(pickerCellSize(400)).toBeGreaterThan(pickerCellSize(250));
  });
  it("stays within its own limits at extreme widths", () => {
    expect(pickerCellSize(0)).toBe(PICKER_CELL_MIN);
    expect(pickerCellSize(-50)).toBe(PICKER_CELL_MIN);
    expect(pickerCellSize(99999)).toBe(PICKER_CELL_MAX);
    expect(pickerCellSize(NaN)).toBe(PICKER_CELL_MIN);
  });
  it("is a whole number", () => {
    expect(Number.isInteger(pickerCellSize(317))).toBe(true);
  });
});
