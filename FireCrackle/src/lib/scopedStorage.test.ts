import { expect, it } from "vitest";
import { scopedStorage } from "./scopedStorage";
import { DEFAULT_MASK_STATE, readMaskState, writeMaskState } from "./maskState";
import { DEFAULT_RULER, readRuler, writeRuler } from "./ruler";
import { readPreferences, writePreferences } from "./preferences";
it("keeps photo geometry and tool state independent of video and other photos", () => {
  const values = new Map<string, string>();
  const video = { getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => { values.set(key, value); } };
  const a = scopedStorage("image:a", video), b = scopedStorage("image:b", video);
  writeMaskState({ ...DEFAULT_MASK_STATE, manualEnabled: true, manualRow: 9 }, video);
  writeMaskState({ ...DEFAULT_MASK_STATE, hudEnabled: true, hudRow: 23 }, a);
  writeRuler({ ...DEFAULT_RULER, enabled: true }, a);
  writePreferences({ matchOpen: true }, a);
  expect(readMaskState(video)).toMatchObject({ manualEnabled: true, manualRow: 9, hudEnabled: false });
  expect(readMaskState(a)).toMatchObject({ hudEnabled: true, hudRow: 23, manualEnabled: false });
  expect(readMaskState(b)).toEqual(DEFAULT_MASK_STATE);
  expect(readRuler(video).enabled).toBe(false); expect(readRuler(a).enabled).toBe(true);
  expect(readPreferences(video).matchOpen).toBe(false); expect(readPreferences(a).matchOpen).toBe(true);
});
