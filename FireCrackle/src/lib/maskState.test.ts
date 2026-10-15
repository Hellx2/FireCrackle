import { describe, expect, it } from "vitest";
import { DEFAULT_MASK_STATE, MASK_STATE_KEY, readMaskState, writeMaskState, type MaskState } from "./maskState";
function memory() {
  const items = new Map<string, string>();
  return { getItem: (key: string) => items.get(key) ?? null, setItem: (key: string, value: string) => { items.set(key, value); } };
}
describe("saved mask state", () => {
  it("starts at 1080p but preserves an explicitly saved resolution", () => {
    expect(readMaskState(memory()).compressionHeight).toBe(1080);
    const storage = memory();
    writeMaskState({ ...DEFAULT_MASK_STATE, compressionHeight: 720 }, storage);
    expect(readMaskState(storage).compressionHeight).toBe(720);
  });
  it("restores both masks' choices, off-frame geometry and compression options", () => {
    const storage = memory();
    const saved: MaskState = { ...DEFAULT_MASK_STATE, hudRow: 31, hudFov: 60, manualEnabled: true, manualQuad: [[-1, -1], [2, -1], [2, 2], [-1, 2]], manualSheet: 0, manualRow: 15, manualMirrored: true, manualOpacity: 0.35, pixelGrid: true, compressionEnabled: true, compressionHeight: 1080, compressionQuality: 0.15 };
    expect(writeMaskState(saved, storage)).toBe(true);
    expect(readMaskState(storage)).toEqual(saved);
  });
  it("repairs invalid fields and folded quads without losing valid choices", () => {
    const storage = memory();
    storage.setItem(MASK_STATE_KEY, JSON.stringify({ hudRow: 32, hudFov: 80, manualEnabled: true, hudEnabled: true, manualSheet: 9, manualRow: 6, manualOpacity: -1, compressionHeight: 5, manualQuad: [[0, 0], [1, 1], [0, 1], [1, 0]] }));
    const restored = readMaskState(storage);
    expect(restored).toEqual({ ...DEFAULT_MASK_STATE, manualEnabled: true, manualRow: 6 });
    storage.setItem(MASK_STATE_KEY, JSON.stringify({ manualQuad: [null, null, null, null] }));
    expect(readMaskState(storage)).toEqual(DEFAULT_MASK_STATE);
  });
  it("handles corrupt JSON and unavailable storage", () => {
    expect(readMaskState({ getItem: () => "{" })).toEqual(DEFAULT_MASK_STATE);
    expect(readMaskState({ getItem: () => { throw Error("blocked"); } })).toEqual(DEFAULT_MASK_STATE);
    expect(writeMaskState(DEFAULT_MASK_STATE, { setItem: () => { throw Error("full"); } })).toBe(false);
  });
});
