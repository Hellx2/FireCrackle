import { beforeEach, expect, it, vi } from "vitest";
import type { ReactElement } from "react";

const hooks = vi.hoisted(() => ({ refs: [] as Array<{ current: unknown }> }));
vi.mock("react", async original => ({
  ...await original<typeof import("react")>(),
  useRef: (current: unknown) => { const ref = { current }; hooks.refs.push(ref); return ref; },
  useState: (value: unknown) => [value, vi.fn()],
  useEffect: vi.fn(),
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
function render() {
  const onChange = vi.fn();
  const svg = VideoRuler({ width: 100, height: 100, zoom: 2, points: [[0.3, 0.5], [0.4, 0.5]], onChange })!;
  hooks.refs[0].current = { getScreenCTM: () => ({ a: 2, b: 0, c: 0, d: 2, e: 100, f: 50 }) };
  return { svg, onChange, points: find(svg, "ruler-calibration"), line: find(svg, "ruler-hit")[0] };
}
beforeEach(() => { hooks.refs = []; });
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
