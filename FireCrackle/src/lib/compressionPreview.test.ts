import { afterEach, describe, expect, it, vi } from "vitest";
import { compressionGeometry, subsampleChroma420, compressionSvg, renderCompressedQuad } from "./compressionPreview";
import type { ManualQuad } from "./quadMask";

const quad: ManualQuad = [[0.4, 0.4], [0.42, 0.4], [0.42, 0.44], [0.4, 0.44]];
describe("compression at delivery pixels", () => {
  it("maps a 2% wide quad to 38.4 pixels at 1920, independently of editor zoom", () => {
    const g = compressionGeometry(2560, 1440, 1080, quad);
    expect(g.width).toBe(1920);
    expect(g.height).toBe(1080);
    expect(g.quadWidth).toBeCloseTo(38.4);
    expect(g.quadHeight).toBeCloseTo(43.2);
    expect(compressionGeometry(2560, 1440, 540, quad).quadWidth).toBeCloseTo(19.2);
  });
  it("preserves aspect ratio, never upscales a decoded frame, and permits off-frame quads", () => {
    expect(compressionGeometry(640, 360, 1080, quad)).toMatchObject({ width:640, height:360 });
    const outside: ManualQuad = [[-0.1,0], [0.1,0], [0.1,1], [-0.1,1]];
    expect(compressionGeometry(1920,1080,720,outside).quadWidth).toBeCloseTo(256);
    expect(compressionGeometry(1080,1920,720,quad)).toMatchObject({ width:405,height:720 });
  });
  it("spreads colour across a 2x2 pixel cell while retaining luminance detail and opaque alpha", () => {
    const pixels = new Uint8ClampedArray([255,0,0,255, 0,0,0,255, 0,0,0,255, 0,0,0,255]);
    subsampleChroma420(pixels,2,2);
    expect(pixels[4]).toBeGreaterThan(20);
    expect(pixels[0]).toBeGreaterThan(pixels[4]);
    expect([pixels[3],pixels[7],pixels[11],pixels[15]]).toEqual([255,255,255,255]);
  });
  it("handles odd frame edges without corrupting neutral colours", () => {
    const pixels = new Uint8ClampedArray(3*3*4).fill(128);
    subsampleChroma420(pixels,3,3);
    for(let i=0;i<pixels.length;i+=4) expect([...pixels.slice(i,i+3)]).toEqual([128,128,128]);
  });
  it("rasterizes projected texels in delivery coordinates, with clipping and no editor transform", () => {
    const svg = compressionSvg([{points:quad,colour:"rgb(255, 0, 0)",alpha:1}],1920,1080);
    expect(svg).toContain('width="1920" height="1080"');
    expect(svg).toContain('points="768,432 806.4,432 806.4,475.2 768,475.2"');
    expect(svg).toContain('shape-rendering="crispEdges"');
    expect(svg).not.toContain("transform=");
  });
});

afterEach(()=> {vi.unstubAllGlobals();vi.restoreAllMocks();});

function mockCanvas() {
  class LoadedImage {
    onload: (()=>void)|null=null;
    set src(_url:string) { queueMicrotask(()=>this.onload?.()); }
  }
  vi.stubGlobal("Image",LoadedImage);
  const ctx={globalAlpha:1, drawImage:vi.fn(function(this:{globalAlpha:number}) { draws.push(this.globalAlpha); }),
    getImageData:vi.fn(()=>({data:new Uint8ClampedArray(16).fill(255)})),putImageData:vi.fn()};
  const draws:number[]=[];
  const canvas={width:0,height:0,getContext:()=>ctx,
    toBlob:vi.fn((cb:(blob:Blob)=>void)=>cb(new Blob(["encoded"],{type:"image/jpeg"})))};
  vi.stubGlobal("document",{createElement:()=>canvas});
  vi.spyOn(URL,"createObjectURL").mockReturnValue("blob:sprite");
  vi.spyOn(URL,"revokeObjectURL").mockImplementation(()=>{});
  return {canvas,ctx,draws};
}

it("composites the scene and translucent fire at delivery size BEFORE encoding",async()=> {
  const {canvas,ctx,draws}=mockCanvas();
  const output=await renderCompressedQuad("blob:frame",[],2,2,0.6,0.15);
  expect([canvas.width,canvas.height]).toEqual([2,2]);
  expect(draws).toEqual([1,0.6]);
  expect(ctx.drawImage.mock.calls[0]).toHaveLength(5);
  expect(ctx.putImageData.mock.invocationCallOrder[0]).toBeLessThan(canvas.toBlob.mock.invocationCallOrder[0]);
  expect(canvas.toBlob).toHaveBeenCalledWith(expect.any(Function),"image/jpeg",0.15);
  expect(output.type).toBe("image/jpeg");
  expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:sprite");
});

it("reports encoder failures instead of silently presenting a sharp preview as compressed",async()=> {
  const {canvas}=mockCanvas();
  canvas.toBlob.mockImplementation(cb=>cb(new Blob(["fallback"],{type:"image/png"})));
  await expect(renderCompressedQuad("blob:frame",[],2,2,1,0.45)).rejects.toThrow("JPEG compression unavailable");
  expect(URL.revokeObjectURL).toHaveBeenCalledWith("blob:sprite");
});
