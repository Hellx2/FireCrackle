import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import { FireQuadMask } from "./FireQuadMask";

it("keeps off-frame handles outside the group that clips the fire texture", () => {
  const markup = renderToStaticMarkup(createElement(FireQuadMask, {
    width:1000, height:500, sheet:1, row:0, mirrored:false, opacity:0.6,
    quad:[[-0.2,-0.1],[1.2,-0.1],[1.2,1.1],[-0.2,1.1]], onQuad:() => {},
  }));
  expect(markup).toContain('cx="-200" cy="-50"');
  expect(markup).toContain('cx="1200" cy="550"');
  expect(markup).toMatch(/clip-path="url\(#[^)]+\)"><g shape-rendering="crispEdges"><\/g><\/g><polygon class="fire-quad-guide"/);
  expect(markup).toContain('<rect width="1000" height="500"');
});
