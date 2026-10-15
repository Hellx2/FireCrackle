import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ReactElement } from "react";

const hooks = vi.hoisted(() => ({ refs: [] as Array<{ current: unknown }>, effects: [] as Array<() => void | (() => void)> }));
vi.mock("react", async original => ({
  ...await original<typeof import("react")>(),
  useRef: (current: unknown) => { const ref = { current }; hooks.refs.push(ref); return ref; },
  useState: (value: unknown) => [value, vi.fn()],
  useEffect: (effect: () => void | (() => void)) => { hooks.effects.push(effect); },
}));
import { VideoRuler } from "./VideoRuler";
type Element = ReactElement<Record<string, any>>;
function find(node: Element, className: string): Element[] {
  const children = [node.props.children].flat(Infinity).filter((child): child is Element => !!child && typeof child === "object" && "props" in child);
  return [...(node.props.className?.split(" ").includes(className) ? [node] : []), ...children.flatMap(child => find(child, className))];
}
function pointer(clientX: number, clientY: number) {
  return { button: 0, pointerId: 1, clientX, clientY, preventDefault: vi.fn(), stopPropagation: vi.fn(), currentTarget: { setPointerCapture: vi.fn() } };
}
function render(locked = false) {
  const onChange = vi.fn();
  const svg = VideoRuler({ width: 100, height: 100, zoom: 2, points: [[0.3, 0.5], [0.4, 0.5]], locked, onChange })!;
  hooks.refs[0].current = { getScreenCTM: () => ({ a: 2, b: 0, c: 0, d: 2, e: 100, f: 50 }), contains: () => false };
  return { svg, onChange, points: find(svg, "ruler-calibration"), line: find(svg, "ruler-hit")[0] };
}
beforeEach(() => { hooks.refs = []; hooks.effects = []; });
afterEach(() => vi.unstubAllGlobals());
it("shows exactly two calibration points while its axis extends beyond both", () => {
  const { svg, points, line } = render();
  expect(points).toHaveLength(2);
  expect(points.map(point => point.props.className)).toEqual(["ruler-calibration point-0", "ruler-calibration point-1"]);
  expect(line.props.x1).toBeLessThan(0);
  expect(line.props.x2).toBeGreaterThan(10);
  expect(svg.props["aria-label"]).toContain("0 and 1");
});
it("recalibrates point 1 in video coordinates through pan and zoom", () => {
  const { svg, points, onChange } = render();
  const down = pointer(180, 150);
  points[1].props.onPointerDown(down);
  expect(down.currentTarget.setPointerCapture).toHaveBeenCalledWith(1);
  expect(down.stopPropagation).toHaveBeenCalled();
  svg.props.onPointerMove(pointer(200, 170));
  const updated = onChange.mock.calls[0][0];
  expect(updated[0]).toEqual([0.3, 0.5]);
  expect(updated[1][0]).toBeCloseTo(0.5);
  expect(updated[1][1]).toBeCloseTo(0.6);
  svg.props.onPointerUp(pointer(200, 170));
  svg.props.onPointerMove(pointer(220, 180));
  expect(onChange).toHaveBeenCalledTimes(1);
});
it("drags the whole axis without changing calibration spacing", () => {
  const { svg, line, onChange } = render();
  line.props.onPointerDown(pointer(200, 150));
  svg.props.onPointerMove(pointer(240, 170));
  const updated = onChange.mock.calls[0][0];
  expect(updated[0][0]).toBeCloseTo(0.5);
  expect(updated[1][0]).toBeCloseTo(0.6);
  expect(updated[0][1]).toBeCloseTo(0.6);
  expect(updated[1][1]).toBeCloseTo(0.6);
});
it.each([false, true])("locks unit via Ctrl or the toggle (toggle=%s)", locked => {
  const { svg, points, onChange } = render(locked);
  points[1].props.onPointerDown({ ...pointer(180, 150), ctrlKey: !locked });
  svg.props.onPointerMove(pointer(160, 230));
  const changed = onChange.mock.calls[0][0];
  expect(changed[0]).toEqual([0.3, 0.5]);
  expect(changed[1][0]).toBeCloseTo(0.3);
  expect(changed[1][1]).toBeCloseTo(0.6);
});
it("Alt places only a selected point and never follows pointer movement", () => {
  const { svg, points, onChange } = render();
  let place: (e: any) => void = () => {};
  vi.stubGlobal("window", { addEventListener: (_: string, listener: typeof place) => { place = listener; }, removeEventListener: vi.fn() });
  hooks.effects[1]();
  place({ ...pointer(220, 170), altKey: true });
  expect(onChange).not.toHaveBeenCalled();
  points[1].props.onPointerDown(pointer(180, 150));
  svg.props.onPointerUp(pointer(180, 150));
  place({ ...pointer(340, 170), altKey: true });
  const changed = onChange.mock.calls[0][0];
  expect(changed[0]).toEqual([0.3, 0.5]);
  expect(changed[1][0]).toBeCloseTo(1.2);
  expect(changed[1][1]).toBeCloseTo(0.6);
  svg.props.onPointerMove(pointer(350, 200));
  expect(onChange).toHaveBeenCalledTimes(1);
  place({ ...pointer(160, 150), altKey: false });
  place({ ...pointer(250, 170), altKey: true });
  expect(onChange).toHaveBeenCalledTimes(1);
});
