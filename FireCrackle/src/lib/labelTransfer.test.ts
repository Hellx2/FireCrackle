import { afterEach, beforeEach, expect, it, vi } from "vitest";
const native = vi.hoisted(() => ({ open: vi.fn(), save: vi.fn(), invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: native.open, save: native.save }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke }));
import { createBackend } from "./api";

beforeEach(() => {
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  vi.clearAllMocks();
});
afterEach(() => vi.unstubAllGlobals());

it("exports to the chosen file and returns the updated view", async () => {
  native.save.mockResolvedValue("C:/labels.json");
  const view = { current: 12, labelled_count: 7 };
  native.invoke.mockResolvedValue(view);
  const backend = await createBackend();
  expect(await backend.transferLabels!("export")).toBe(view);
  expect(native.save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "firecrackle-labels.json" }));
  expect(native.invoke).toHaveBeenCalledExactlyOnceWith("export_labels", { path: "C:/labels.json" });
});

it("imports the selected file through the workspace", async () => {
  native.open.mockResolvedValue("C:/labels.json");
  native.invoke.mockResolvedValue({ labelled_count: 10 });
  const backend = await createBackend();
  expect(await backend.transferLabels!("import")).toEqual({ labelled_count: 10 });
  expect(native.invoke).toHaveBeenCalledExactlyOnceWith("import_labels", { path: "C:/labels.json" });
});

it("cancellation never invokes a label command", async () => {
  native.open.mockResolvedValue(null); native.save.mockResolvedValue(null);
  const backend = await createBackend();
  expect(await backend.transferLabels!("import")).toBeNull();
  expect(await backend.transferLabels!("export")).toBeNull();
  expect(native.invoke).not.toHaveBeenCalled();
});

it("reports an import failure to the caller", async () => {
  native.open.mockResolvedValue("C:/bad.json");
  native.invoke.mockRejectedValue(new Error("Wrong video"));
  const backend = await createBackend();
  await expect(backend.transferLabels!("import")).rejects.toThrow("Wrong video");
});
