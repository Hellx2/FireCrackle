import { afterEach, expect, it, vi } from "vitest";
import { proofFilename, proofLayout, proofOpacity, saveComparisonProof } from "./comparisonProof";
const native = vi.hoisted(() => ({ save: vi.fn(), invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: native.save }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke }));
afterEach(() => { vi.unstubAllGlobals(); vi.clearAllMocks(); });

it("keeps tiny captions readable and bounds huge photos without distorting them", () => {
  const tiny = proofLayout(16, 16); expect(tiny.panelWidth).toBeGreaterThanOrEqual(360);
  const huge = proofLayout(16000, 8000); expect(huge.panelWidth).toBe(4096); expect(huge.panelHeight).toBe(2048);
  expect(() => proofLayout(0, 1080)).toThrow("not ready");
});
it("shows a temporarily hidden mask in proof and keeps a visible mask's chosen opacity", () => {
  expect(proofOpacity(0)).toBe(1); expect(proofOpacity(0.6)).toBe(0.6);
  expect(proofFilename('photo:one.png')).toBe("photo-one-comparison-proof.png");
});
it("does not write anything when the native save dialog is cancelled", async () => {
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} }); native.save.mockResolvedValue(null);
  expect(await saveComparisonProof(new Blob(["picture"]), "proof.png")).toBe(false);
  expect(native.invoke).not.toHaveBeenCalled();
});
it("sends the generated PNG bytes to the native save command and surfaces write failures", async () => {
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} }); native.save.mockResolvedValue("C:/proof.png"); native.invoke.mockResolvedValue(undefined);
  expect(await saveComparisonProof(new Blob([new Uint8Array([137, 80, 78, 71])]), "proof.png")).toBe(true);
  expect(native.invoke).toHaveBeenCalledWith("save_comparison_proof", { path: "C:/proof.png", png: [137, 80, 78, 71] });
  native.invoke.mockRejectedValue(Error("Disk full"));
  await expect(saveComparisonProof(new Blob(["image"]), "proof.png")).rejects.toThrow("Disk full");
});
