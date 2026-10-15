import { beforeEach, describe, expect, it, vi } from "vitest";

// Exercise effect setup/cleanup and ref persistence without a browser DOM.
const hooks = vi.hoisted(() => ({
  refs: [] as Array<{ current: unknown }>,
  cursor: 0,
  effects: [] as Array<{ setup: () => void | (() => void); deps?: unknown[] }>,
}));
vi.mock("react", () => ({
  useRef: (initial: unknown) => {
    const index = hooks.cursor++;
    return (hooks.refs[index] ??= { current: initial });
  },
  useEffect: (setup: () => void | (() => void), deps?: unknown[]) => hooks.effects.push({ setup, deps }),
}));
import { useWheel } from "./useWheel";

function render(ref: { current: HTMLElement | null }, handler: (event: WheelEvent) => boolean) {
  hooks.cursor = 0;
  hooks.effects = [];
  useWheel(ref, handler);
}

function wheel(element: EventTarget) {
  const event = new Event("wheel", { cancelable: true });
  element.dispatchEvent(event);
  return event;
}

beforeEach(() => {
  hooks.refs = [];
  hooks.cursor = 0;
  hooks.effects = [];
});

describe("wheel listener lifecycle", () => {
  it("survives StrictMode setup, cleanup, then setup on the same element", () => {
    const element = new EventTarget() as HTMLElement;
    const handler = vi.fn(() => true);
    render({ current: element }, handler);
    const cleanups = hooks.effects.map(({ setup }) => setup());
    expect(wheel(element).defaultPrevented).toBe(true);
    cleanups.forEach((cleanup) => cleanup?.());
    expect(wheel(element).defaultPrevented).toBe(false);
    hooks.effects.forEach(({ setup }) => setup());
    expect(wheel(element).defaultPrevented).toBe(true);
    expect(handler).toHaveBeenCalledTimes(2);
  });

  it("uses the newest handler once and leaves unhandled scrolling alone", () => {
    const element = new EventTarget() as HTMLElement;
    const ref = { current: element };
    const first = vi.fn(() => true);
    render(ref, first);
    hooks.effects.forEach(({ setup }) => setup());
    const next = vi.fn(() => false);
    render(ref, next);
    hooks.effects.filter(({ deps }) => !deps).forEach(({ setup }) => setup());
    expect(wheel(element).defaultPrevented).toBe(false);
    expect(first).not.toHaveBeenCalled();
    expect(next).toHaveBeenCalledTimes(1);
  });

  it("binds a late-mounted element, follows replacements and cleans up", () => {
    const ref: { current: HTMLElement | null } = { current: null };
    const handler = vi.fn(() => true);
    render(ref, handler);
    const cleanups = hooks.effects.map(({ setup }) => setup());
    const old = new EventTarget() as HTMLElement;
    ref.current = old;
    render(ref, handler);
    hooks.effects.filter(({ deps }) => !deps).forEach(({ setup }) => setup());
    expect(wheel(old).defaultPrevented).toBe(true);
    const next = new EventTarget() as HTMLElement;
    ref.current = next;
    render(ref, handler);
    hooks.effects.filter(({ deps }) => !deps).forEach(({ setup }) => setup());
    expect(wheel(old).defaultPrevented).toBe(false);
    expect(wheel(next).defaultPrevented).toBe(true);
    cleanups.forEach((cleanup) => cleanup?.());
    expect(wheel(next).defaultPrevented).toBe(false);
    expect(handler).toHaveBeenCalledTimes(2);
  });
});
