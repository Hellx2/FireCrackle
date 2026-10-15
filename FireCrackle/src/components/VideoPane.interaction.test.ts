import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
const hooks = vi.hoisted(() => ({ states: [] as unknown[], refs: [] as Array<{ current: unknown }>, stateCursor: 0, refCursor: 0, effects: [] as Array<() => void | (() => void)> }));
vi.mock("react", async (original) => ({
  ...await original<typeof import("react")>(),
  useState: (initial: unknown) => {
    const i = hooks.stateCursor++;
    if (!(i in hooks.states)) hooks.states[i] = initial;
    return [hooks.states[i], (next: unknown) => { hooks.states[i] = typeof next === "function" ? next(hooks.states[i]) : next; }];
  },
  useRef: (initial: unknown) => hooks.refs[hooks.refCursor++] ??= { current: initial },
  useEffect: (setup: () => void | (() => void)) => { hooks.effects.push(setup); },
  useImperativeHandle: () => {},
}));
vi.mock("../hooks/useWheel", () => ({ useWheel: () => {} }));
import { VideoPane } from "./VideoPane";
import { createMockBackend } from "../lib/mock";
import { DEFAULT_MASK_STATE, MASK_STATE_KEY, readMaskState } from "../lib/maskState";

type Element = ReactElement<Record<string, any>>;
function viewport(node: Element): Element | undefined {
  if (node.props.className === "video-frame zoomed") return node;
  for (const child of [node.props.children].flat(Infinity)) {
    if (child && typeof child === "object" && "props" in child) {
      const match = viewport(child);
      if (match) return match;
    }
  }
}
async function pane() {
  hooks.stateCursor = hooks.refCursor = 0;
  hooks.effects = [];
  const backend = createMockBackend();
  return viewport(VideoPane({ backend, view: await backend.view(), onError: vi.fn() }))!;
}
function pointer(x: number, y: number, button = 0) {
  return { clientX: x, clientY: y, button, pointerId: 7, preventDefault: vi.fn(), currentTarget: { setPointerCapture: vi.fn() } };
}
beforeEach(() => { hooks.states = []; hooks.refs = []; });
afterEach(() => vi.unstubAllGlobals());

it("restores saved mask settings and persists a keyboard parallelogram edit", async () => {
  let stored = JSON.stringify({ ...DEFAULT_MASK_STATE, manualEnabled: true, hudRow: 19, hudFov: 60, manualQuad: [[0.1, 0.1], [0.8, 0.2], [0.7, 0.9], [0.2, 0.7]] });
  vi.stubGlobal("localStorage", { getItem: () => stored, setItem: (key: string, value: string) => { expect(key).toBe(MASK_STATE_KEY); stored = value; } });
  let keyHandler: ((e: KeyboardEvent) => void) | undefined;
  vi.stubGlobal("window", { addEventListener: (_: string, listener: (e: KeyboardEvent) => void) => { keyHandler = listener; }, removeEventListener: vi.fn() });
  await pane();
  hooks.effects[0](); // Save restored state without resetting either preview's choices.
  expect(readMaskState().hudRow).toBe(19);
  expect(readMaskState().hudFov).toBe(60);
  const stop = hooks.effects[1]();
  const key = { key: "p", target: { tagName: "BUTTON" }, preventDefault: vi.fn() } as unknown as KeyboardEvent;
  keyHandler!(key);
  expect(key.preventDefault).toHaveBeenCalledOnce();
  await pane(); hooks.effects[0]();
  const q = readMaskState().manualQuad;
  expect(q[0][0] + q[2][0]).toBeCloseTo(q[1][0] + q[3][0]);
  expect(q[0][1] + q[2][1]).toBeCloseTo(q[1][1] + q[3][1]);
  stop?.();
});

it("suppresses native dragging and pans with captured pointer movement", async () => {
  const frame = await pane();
  const down = pointer(100, 150);
  frame.props.onPointerDown(down);
  expect(down.preventDefault).toHaveBeenCalledOnce();
  expect(down.currentTarget.setPointerCapture).toHaveBeenCalledExactlyOnceWith(7);
  frame.props.onPointerMove(pointer(140, 130));
  expect(hooks.states).toContainEqual({ scale: 1, x: 40, y: -20 });
  frame.props.onPointerUp();
  frame.props.onPointerMove(pointer(250, 250));
  expect(hooks.states).toContainEqual({ scale: 1, x: 40, y: -20 });
  const drag = { preventDefault: vi.fn() };
  frame.props.onDragStart(drag);
  expect(drag.preventDefault).toHaveBeenCalledOnce();
  expect(frame.props.draggable).toBe(false);
});

it("leaves non-primary buttons alone", async () => {
  const frame = await pane();
  const down = pointer(10, 10, 2);
  frame.props.onPointerDown(down);
  frame.props.onPointerMove(pointer(50, 50));
  expect(down.preventDefault).not.toHaveBeenCalled();
  expect(down.currentTarget.setPointerCapture).not.toHaveBeenCalled();
  expect(hooks.states).toContainEqual({ scale: 1, x: 0, y: 0 });
});
