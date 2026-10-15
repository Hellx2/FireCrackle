import { beforeEach, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
const hooks = vi.hoisted(() => ({ states: [] as unknown[], refs: [] as Array<{ current: unknown }>, stateCursor: 0, refCursor: 0 }));
vi.mock("react", async (original) => ({
  ...await original<typeof import("react")>(),
  useState: (initial: unknown) => {
    const i = hooks.stateCursor++;
    if (!(i in hooks.states)) hooks.states[i] = initial;
    return [hooks.states[i], (next: unknown) => { hooks.states[i] = typeof next === "function" ? next(hooks.states[i]) : next; }];
  },
  useRef: (initial: unknown) => hooks.refs[hooks.refCursor++] ??= { current: initial },
  useEffect: () => {},
  useImperativeHandle: () => {},
}));
vi.mock("../hooks/useWheel", () => ({ useWheel: () => {} }));
import { VideoPane } from "./VideoPane";
import { createMockBackend } from "../lib/mock";

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
  const backend = createMockBackend();
  return viewport(VideoPane({ backend, view: await backend.view(), onError: vi.fn() }))!;
}
function pointer(x: number, y: number, button = 0) {
  return { clientX: x, clientY: y, button, pointerId: 7, preventDefault: vi.fn(), currentTarget: { setPointerCapture: vi.fn() } };
}
beforeEach(() => { hooks.states = []; hooks.refs = []; });

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
