import { expect, it } from "vitest";
import { matchFireQuad } from "./quadMatch";
import { rasterizeFire } from "./quadRaster";
import type { ManualQuad } from "./quadMask";

function sheets():[Uint8ClampedArray,Uint8ClampedArray] {
  return [0,1].map(sheet=> {
    let seed=sheet+19;
    const data=new Uint8ClampedArray(16*512*4);
    for(let y=0;y<512;y++)for(let x=0;x<16;x++) {
      seed=(Math.imul(seed,1664525)+1013904223)>>>0;
      const bright=(seed>>>24)/255,i=(y*16+x)*4;
      data.set([120+135*bright,45+200*bright,15+220*bright,(y%16>4||x>3&&x<12)?255:0],i);
    }
    return data;
  }) as [Uint8ClampedArray,Uint8ClampedArray];
}
const q:ManualQuad=[[0.2,0.1],[0.8,0.1],[0.8,0.9],[0.2,0.9]];
function frame(sheet:Uint8ClampedArray,row:number,mirrored:boolean,quad=q,width=160,height=100) {
  const data=rasterizeFire(sheet,row,mirrored,quad,width,height);
  for(let i=0;i<data.length;i+=4)if(!data[i+3])data.set([20,10,5,255],i);
  return {data,width,height};
}

it("finds the selected sheet, row and mirror among all128 possibilities",()=> {
  const atlas=sheets(),result=matchFireQuad(frame(atlas[1],23,true),atlas,q);
  expect(result.searched).toBe(128);
  expect(result.candidates[0]).toMatchObject({sheet:1,row:23,mirrored:true});
  expect(result.status).toBe("clear");
});
it("tolerates brightness changes without mistaking the other sheet",()=> {
  const atlas=sheets(),image=frame(atlas[0],6,false);
  for(let i=0;i<image.data.length;i+=4)for(let c=0;c<3;c++)image.data[i+c]=image.data[i+c]*0.65+7;
  const result=matchFireQuad(image,atlas,q);
  expect(result.candidates[0]).toMatchObject({sheet:0,row:6,mirrored:false});
  expect(result.status).toBe("clear");
});
it("can refine a slightly displaced rough outline",()=> {
  const atlas=sheets();
  const rough=q.map(([x,y])=>[x+0.03,y]) as ManualQuad;
  const result=matchFireQuad(frame(atlas[0],12,true),atlas,rough);
  expect(result.candidates[0]).toMatchObject({sheet:0,row:12,mirrored:true});
  expect(result.candidates[0].quad[0][0]).toBeCloseTo(q[0][0],5);
});
it("does not automatically select a winner when two sheets are indistinguishable",()=> {
  const [atlas]=sheets();
  const result=matchFireQuad(frame(atlas,4,false),[atlas,atlas],q);
  expect(result.status).toBe("ambiguous");
  expect(result.margin).toBeCloseTo(0);
});
it("abstains on a flat background, tiny patch or entirely off-frame quad",()=> {
  const atlas=sheets(),blank={width:160,height:100,data:new Uint8ClampedArray(160*100*4).fill(100)};
  expect(matchFireQuad(blank,atlas,q).status).toBe("weak");
  const tiny:ManualQuad=[[0.1,0.1],[0.11,0.1],[0.11,0.12],[0.1,0.12]];
  expect(matchFireQuad(blank,atlas,tiny).status).toBe("insufficient");
  const outside=q.map(([x,y])=>[x+2,y]) as ManualQuad;
  expect(matchFireQuad(blank,atlas,outside).status).toBe("insufficient");
});

it("builds a colour mask first and matches visible fire despite dark particles covering half the quad",()=> {
  const atlas=sheets(),image=frame(atlas[1],23,true);
  for(let y=10;y<90;y++)for(let x=32;x<80;x++)image.data.set([8,8,8,255],(y*image.width+x)*4);
  const result=matchFireQuad(image,atlas,q);
  expect(result.candidates[0]).toMatchObject({sheet:1,row:23,mirrored:true});
  expect(result.status).toBe("clear");
  expect(result.candidates[0].support).toBeGreaterThanOrEqual(32);
  expect(result.candidates[0].maskSamples).toBeLessThan(180);
});
it("abstains when almost all fire is covered rather than inventing a row",()=> {
  const atlas=sheets(),image=frame(atlas[0],12,false);
  for(let y=10;y<90;y++)for(let x=32;x<122;x++)image.data.set([8,8,8,255],(y*image.width+x)*4);
  expect(matchFireQuad(image,atlas,q).status).not.toBe("clear");
});

it("adds positive evidence without diluting existing votes with masked colour mismatches",()=> {
  const atlas=sheets(),covered=frame(atlas[1],23,true);
  for(let y=10;y<90;y++)for(let x=32;x<80;x++)covered.data.set([8,8,8,255],(y*160+x)*4);
  const partial=matchFireQuad(covered,atlas,q).candidates.find(c=>c.sheet===1&&c.row===23&&c.mirrored)!;
  expect(partial).toBeDefined();
  // Reveal unrelated but fire-coloured pixels on the hidden side. They may
  // support rivals, but cannot subtract this candidate's existing evidence.
  const unrelated=frame(atlas[0],3,false);
  for(let y=10;y<90;y++)for(let x=32;x<80;x++)covered.data.set(unrelated.data.subarray((y*160+x)*4,(y*160+x)*4+4),(y*160+x)*4);
  const extra=matchFireQuad(covered,atlas,q).candidates.find(c=>c.sheet===1&&c.row===23&&c.mirrored)!;
  expect(extra).toBeDefined();expect(extra.score).toBeGreaterThanOrEqual(partial.score);
  const full=matchFireQuad(frame(atlas[1],23,true),atlas,q).candidates[0];
  expect(full.score).toBeGreaterThan(partial.score);
});
