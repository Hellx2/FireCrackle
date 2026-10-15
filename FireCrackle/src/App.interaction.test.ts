import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
const fixture = vi.hoisted(() => ({
  backend: { mock: true }, act: vi.fn(async (_action: unknown) => {}), view: {} as any,
  states: [] as any[], refs: [] as Array<{current: any}>, stateCursor: 0, refCursor: 0, effectCursor: 0,
  effects: [] as Array<{ deps?: unknown[]; cleanup?: void | (() => void) }>, pending: [] as Array<() => void>,
}));
vi.mock("react", async original => ({
  ...await original<typeof import("react")>(),
  useState: (initial: unknown) => {
    const i = fixture.stateCursor++;
    if (!(i in fixture.states)) fixture.states[i] = typeof initial === "function" ? initial() : initial;
    return [fixture.states[i], (next: any) => { fixture.states[i] = typeof next === "function" ? next(fixture.states[i]) : next; }];
  },
  useRef: (initial: unknown) => fixture.refs[fixture.refCursor++] ??= {current: initial},
  useCallback: (fn: unknown) => fn,
  useEffect: (setup: () => void | (() => void), deps?: unknown[]) => {
    const i = fixture.effectCursor++, old = fixture.effects[i];
    if (old && deps && old.deps && deps.length === old.deps.length && deps.every((v, j) => Object.is(v, old.deps![j]))) return;
    fixture.pending.push(() => { old?.cleanup?.(); fixture.effects[i] = { deps, cleanup: setup() }; });
  },
}));
vi.mock("./lib/api", async original => ({ ...await original<typeof import("./lib/api")>(), createBackend: async () => fixture.backend }));
vi.mock("./hooks/useWorkspace", () => ({ useWorkspace: () => ({ view: fixture.view, act: fixture.act, busy: {current: false}, flags: [], settings: [], version: 0, save: vi.fn(), opening: false, clearError: vi.fn() }) }));
vi.mock("./hooks/useLayout", () => ({ useLayout: () => ({ layout: { sideWidth: 600, videoHeight: 400, flagsHeight: 120 }, set: vi.fn(), reset: vi.fn() }) }));
vi.mock("./hooks/usePanelZooms", () => ({ usePanelZooms: () => ({ zooms: {picker: 1, flags: 1}, wheel: vi.fn(), reset: vi.fn() }) }));
vi.mock("./hooks/useWheel", () => ({ useWheel: () => {} }));
import App from "./App";
import { Picker } from "./components/Picker";
import { VideoPane } from "./components/VideoPane";

type Element = ReactElement<Record<string, any>>;
function find(node: Element, test: (e: Element) => boolean): Element | undefined {
  if (test(node)) return node;
  for (const child of [node.props.children].flat(Infinity)) if (child && typeof child === "object" && "props" in child) {
    const match = find(child, test); if (match) return match;
  }
}
function render() {
  fixture.stateCursor = fixture.refCursor = fixture.effectCursor = 0; fixture.pending = [];
  const tree = App() as Element;
  fixture.pending.forEach(run => run());
  return { picker: find(tree, e => e.type === Picker)!, video: find(tree, e => e.type === VideoPane)!, confirm: find(tree, e => e.props.label === "Confirm candidate")!, noFire: find(tree, e => e.props.label === "Choose no fire")! };
}
function key(key: string) {
  window.dispatchEvent(Object.assign(new Event("keydown", {cancelable: true}), {key, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, repeat: false}));
}
beforeEach(() => {
  fixture.act.mockClear(); fixture.states = [fixture.backend, null]; fixture.refs = []; fixture.effects = [];
  fixture.view = { current: 10, frame_count: 100, fps: 30, pts: 1 / 3, label: "unset", pick: {sheet: 1, mirrored: false}, suggestion: {sheet: 1, row: 7, mirrored: false, alt_row: 8, from_frame: 1}, typed_pending: null, labelled_count: 1, flag_count: 0, warnings: [], save_error: null };
  const target = new EventTarget();
  vi.stubGlobal("window", {
    addEventListener: (type: string, listener: EventListener, capture = false) => target.addEventListener(type, listener, {capture}),
    removeEventListener: (type: string, listener: EventListener, capture = false) => target.removeEventListener(type, listener, {capture}),
    dispatchEvent: (event: Event) => target.dispatchEvent(event),
  });
});
afterEach(() => { fixture.effects.forEach(e => e?.cleanup?.()); vi.unstubAllGlobals(); });

it("atlas clicks and preview-only Ctrl clicks never save or advance; Confirm saves the staged choice", () => {
  let ui = render();
  const preview = vi.fn(() => true); ui.video.props.previewRef.current = { selectSprite: preview };
  ui.picker.props.onCell(2, 31, false);
  expect(fixture.act).not.toHaveBeenCalled();
  ui = render(); expect(ui.picker.props.candidate).toEqual({fire: {sheet: 0, row: 31, mirrored: true}});
  ui.picker.props.onCell(1, 9, true);
  expect(fixture.act).not.toHaveBeenCalled();
  ui = render(); expect(ui.picker.props.candidate.fire.row).toBe(31);
  ui.confirm.props.onClick();
  expect(fixture.act).toHaveBeenCalledExactlyOnceWith({type: "clickCell", col: 2, row: 31});
});
it("forward navigation never auto-confirms and Enter confirms typed rows only explicitly", () => {
  render(); key("ArrowRight");
  expect(fixture.act).toHaveBeenCalledExactlyOnceWith({type: "step", frames: 1});
  fixture.act.mockClear(); key("3"); render(); key("1"); render();
  expect(fixture.act).not.toHaveBeenCalled(); key("Enter");
  expect(fixture.act).toHaveBeenCalledExactlyOnceWith({type: "clickCell", col: 1, row: 31});
});
it("No fire is a reviewed marker saved only after confirmation", () => {
  let ui = render(); ui.noFire.props.onClick();
  expect(fixture.act).not.toHaveBeenCalled(); ui = render();
  expect(ui.picker.props.candidate).toBe("no_fire"); ui.confirm.props.onClick();
  expect(fixture.act).toHaveBeenCalledExactlyOnceWith({type: "noFire"});
});
