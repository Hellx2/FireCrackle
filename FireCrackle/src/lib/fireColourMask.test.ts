import { expect, it } from "vitest";
import { firePalette, isFirePaletteColour } from "./fireColourMask";

const palette=firePalette([new Uint8ClampedArray([240,120,12,255,255,250,230,255,0,0,0,0])]);
it("uses opaque palette colours and accepts brightness-scaled orange",()=> {
  expect(palette).toHaveLength(2);
  expect(isFirePaletteColour(240,120,12,palette)).toBe(true);
  expect(isFirePaletteColour(120,60,6,palette)).toBe(true);
  expect(isFirePaletteColour(252,248,226,palette)).toBe(true);
  expect(isFirePaletteColour(178,175,161,palette)).toBe(true);
});
it("does not turn dark smoke, lava fog or grey stone into fire evidence",()=> {
  for(const colour of [[8,8,8],[120,20,3],[75,75,75],[30,100,160]])
    expect(isFirePaletteColour(...colour as [number,number,number],palette)).toBe(false);
});
