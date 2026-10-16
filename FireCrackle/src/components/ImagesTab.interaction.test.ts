import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
import type { Backend } from "../lib/api";
const hooks = vi.hoisted(() => ({ states: [] as any[], refs: [] as any[], stateCursor: 0, refCursor: 0, effects: [] as Array<() => (() => void) | void> }));
vi.mock("react", async original => ({ ...await original<typeof import("react")>(),
  useState: (initial: any) => { const i = hooks.stateCursor++; if (!(i in hooks.states)) hooks.states[i] = typeof initial === "function" ? initial() : initial; return [hooks.states[i], (next: any) => { hooks.states[i] = typeof next === "function" ? next(hooks.states[i]) : next; }]; },
  useRef: (initial: any) => hooks.refs[hooks.refCursor++] ??= { current: initial },
  useEffect: (fn: any) => { hooks.effects.push(fn); }, useCallback: (fn: any) => fn,
}));
vi.mock("../hooks/useWheel", () => ({ useWheel: () => {} }));
import { ImagesTab } from "./ImagesTab";
import { VideoPane } from "./VideoPane";
import { Picker } from "./Picker";
type Element = ReactElement<Record<string, any>>;
const backend = { frame: vi.fn(), act: vi.fn(), openVideo: vi.fn() } as unknown as Backend;
function all(node: any, match: (element: Element) => boolean): Element[] {
  if (Array.isArray(node)) return node.flatMap(value => all(value, match));
  if (!node || typeof node !== "object" || !("props" in node)) return [];
  return [...(match(node) ? [node] : []), ...all(node.props.children, match)];
}
function render(active = true) { hooks.stateCursor = hooks.refCursor = 0; hooks.effects = []; return ImagesTab({ active, backend }); }
function importPhotos(root: any, names: string[]) {
  const input = all(root, element => element.type === "input")[0];
  input.props.onChange({ target: { files: names.map(name => ({ name, type: "image/png", size: 20, lastModified: 1 })), value: "files" } });
}
beforeEach(() => { hooks.states = []; hooks.refs = []; vi.stubGlobal("window", new EventTarget()); vi.stubGlobal("URL", { createObjectURL: vi.fn((file: File) => `blob:${file.name}`), revokeObjectURL: vi.fn() }); });
afterEach(() => vi.unstubAllGlobals());
it("uses the same viewer/tools and picker, with no confirmation or time guess, independently of video", () => {
  let root = render(); importPhotos(root, ["a.png", "b.png"]); root = render();
  let viewer = all(root, element => element.type === VideoPane)[0];
  expect(viewer.props.backend).toBe(backend); expect(viewer.props.stillImage.name).toBe("a.png");
  expect(viewer.props.view.suggestion).toBeNull(); expect(viewer.props.view.label).toBe("unset");
  const picker = all(root, element => element.type === Picker)[0]; expect(picker.props.previewOnly).toBe(true);
  expect(all(root, element => element.props.label === "Confirm candidate")).toHaveLength(0);
  viewer.props.onStep(1); root = render(); viewer = all(root, element => element.type === VideoPane)[0];
  expect(viewer.props.stillImage.name).toBe("b.png");
  expect(render(false)).toBeNull(); root = render();
  expect(all(root, element => element.type === VideoPane)[0].props.stillImage.name).toBe("b.png");
  expect(backend.frame).not.toHaveBeenCalled(); expect(backend.act).not.toHaveBeenCalled(); expect(backend.openVideo).not.toHaveBeenCalled();
});
it("removes photos safely and revokes gallery URLs on clear", () => {
  let root = render(); importPhotos(root, ["a.png", "b.png"]); root = render();
  all(root, element => element.props.label === "Remove selected photo")[0].props.onClick(); root = render();
  expect(all(root, element => element.type === VideoPane)[0].props.stillImage.name).toBe("b.png");
  expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:a.png");
  all(root, element => element.type === "button" && element.props.children === "Clear gallery")[0].props.onClick(); root = render();
  expect(all(root, element => element.type === VideoPane)).toHaveLength(0);
  expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:b.png");
});
it("pastes a clipboard photo into the gallery and only handles paste in the active Images tab", () => {
  let root = render(); importPhotos(root, ["a.png"]); root = render();
  const setup = hooks.effects.find(effect => effect.toString().includes("const paste"))!;
  const stop = setup();
  const image = { name: "clipboard.png", type: "image/png", size: 30, lastModified: 2 };
  const event = Object.assign(new Event("paste", { cancelable: true }), { clipboardData: { files: [image], items: [] } });
  window.dispatchEvent(event); expect(event.defaultPrevented).toBe(true);
  root = render(); expect(all(root, element => element.type === VideoPane)[0].props.stillImage.name).toBe("clipboard.png");
  stop?.(); render(false);
  const hidden = Object.assign(new Event("paste", { cancelable: true }), { clipboardData: { files: [image], items: [] } });
  window.dispatchEvent(hidden); expect(hidden.defaultPrevented).toBe(false);
});
