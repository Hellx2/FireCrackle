import { afterEach, expect, it, vi } from "vitest";
import { copyComparisonProof, proofLayout, proofOpacity } from "./comparisonProof";
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
});
it("copies generated PNG bytes through the native clipboard with no save dialog", async () => {
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} }); native.invoke.mockResolvedValue(undefined);
  await copyComparisonProof(new Blob([new Uint8Array([137,80,78,71])],{type:"image/png"}));
  expect(native.invoke).toHaveBeenCalledWith("copy_comparison_proof",{png:[137,80,78,71]});
  expect(native.save).not.toHaveBeenCalled();
});
it("surfaces clipboard errors without falling back to file saving", async () => {
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });native.invoke.mockRejectedValue(Error("Clipboard busy"));
  await expect(copyComparisonProof(new Blob(["image"]))).rejects.toThrow("Clipboard busy");
  expect(native.save).not.toHaveBeenCalled();
});
it("starts browser image copying while the proof is still rendering",async()=>{
  vi.stubGlobal("window",{});const write=vi.fn(async()=>{});vi.stubGlobal("navigator",{clipboard:{write}});
  const item=vi.fn(function(this:any,data:any){this.data=data;});vi.stubGlobal("ClipboardItem",item);
  const pending=new Promise<Blob>(()=>{});await copyComparisonProof(pending);
  expect(item).toHaveBeenCalledWith({"image/png":pending});expect(write).toHaveBeenCalledOnce();expect(native.save).not.toHaveBeenCalled();
});
it("reports unsupported clipboard rather than downloading a file",async()=>{
  vi.stubGlobal("window",{});vi.stubGlobal("ClipboardItem",undefined);
  await expect(copyComparisonProof(new Blob(["image"]))).rejects.toThrow("clipboard is unavailable");expect(native.save).not.toHaveBeenCalled();
});
