import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
const hooks = vi.hoisted(() => ({ states: [] as unknown[], refs: [] as Array<{current: unknown}>, si: 0, ri: 0, effects: [] as Array<() => void | (() => void)> }));
vi.mock("react", async original => ({
  ...await original<typeof import("react")>(),
  useState: (initial: unknown) => {
    const i = hooks.si++; if (!(i in hooks.states)) hooks.states[i] = initial;
    return [hooks.states[i], (next: unknown) => { hooks.states[i] = next; }];
  },
  useRef: (initial: unknown) => hooks.refs[hooks.ri++] ??= {current: initial},
  useEffect: (effect: () => void | (() => void)) => { hooks.effects.push(effect); },
}));
import { Picker } from "./Picker";
import type { View } from "../lib/api";

type Element = ReactElement<Record<string, any>>;
function all(node: Element, test: (e: Element) => boolean): Element[] {
  const children = [node.props.children].flat(Infinity).filter((x): x is Element => !!x && typeof x === "object" && "props" in x);
  return [...(test(node) ? [node] : []), ...children.flatMap(child => all(child, test))];
}
const onCell = vi.fn();
const view = { label: "unset", suggestion: null } as View;
let cleanup: Array<void | (() => void)> = [];
function render() {
  hooks.si = hooks.ri = 0; hooks.effects = [];
  const tree = Picker({ view, cell: 40, onCell, candidate: {fire: {sheet: 1, row: 0, mirrored: false}} });
  return {
    cells: all(tree, e => e.type === "button" && e.props.className?.startsWith("cell")),
    input: (side: string) => all(tree, e => e.props["aria-label"] === `${side} edge fire pixels`)[0],
    reset: all(tree, e => e.props.label === "Clear edge pixel filters")[0],
  };
}
beforeEach(() => {
  hooks.states = []; hooks.refs = []; onCell.mockClear();
  const data = [new Uint8ClampedArray(16 * 512 * 4), new Uint8ClampedArray(16 * 512 * 4)];
  for (let sheet = 0; sheet < 2; sheet++) {
    for (let y = 0; y < (sheet ? 2 : 3); y++) data[sheet][(y * 16) * 4 + 3] = 255;
    for (let y = 0; y < (sheet ? 9 : 7); y++) data[sheet][(y * 16 + 15) * 4 + 3] = 255;
  }
  vi.stubGlobal("Image", class {
    naturalWidth = 16; naturalHeight = 512; onload?: () => void; onerror?: () => void; source = "";
    set src(value: string) { this.source = value; this.onload?.(); }
  });
  vi.stubGlobal("document", { createElement: () => {
    let sheet = 0;
    return { getContext: () => ({ drawImage: (image: {source: string}) => { sheet = image.source.includes("fire_0") ? 0 : 1; }, getImageData: () => ({data: data[sheet]}) }) };
  } });
});
afterEach(() => { cleanup.forEach(stop => stop?.()); cleanup = []; vi.unstubAllGlobals(); });

it("visually filters exact outermost counts while keeping every cell and click working", async () => {
  render(); cleanup = hooks.effects.map(effect => effect());
  await Promise.resolve(); await Promise.resolve();
  let ui = render();
  expect(ui.cells).toHaveLength(128);
  expect(ui.input("Left").type).toBe("input");
  expect(ui.input("Left").props).toMatchObject({type: "number", min: 0, max: 16, placeholder: "Any", value: ""});
  ui.input("Left").props.onChange({target: {value: "3"}});
  ui.input("Right").props.onChange({target: {value: "7"}});
  ui = render();
  const matching = ui.cells.filter(e => !e.props.className.includes("edge-filter-miss"));
  expect(matching).toHaveLength(1);
  expect(matching[0].props.title).toContain("Edge pixels: left 3 · right 7");
  const dimmed = ui.cells.find(e => e.props["aria-label"].startsWith("sheet 1, row 0, normal"))!;
  expect(dimmed.props.className).toContain("edge-filter-miss");
  expect(dimmed.props.className).toContain("atlas-candidate");
  expect(dimmed.props.disabled).toBeUndefined();
  expect(dimmed.props["aria-disabled"]).toBeUndefined();
  dimmed.props.onClick({ctrlKey: false}); dimmed.props.onClick({ctrlKey: true});
  expect(onCell.mock.calls).toEqual([[1, 0, false], [1, 0, true]]);
  ui.reset.props.onClick();
  expect(render().cells.every(e => !e.props.className.includes("edge-filter-miss"))).toBe(true);
  ui = render();
  ui.input("Left").props.onChange({target: {value: "0"}});
  expect(render().input("Left").props.value).toBe(0);
  ui = render(); ui.input("Left").props.onChange({target: {value: ""}});
  expect(render().cells.every(e => !e.props.className.includes("edge-filter-miss"))).toBe(true);
});
