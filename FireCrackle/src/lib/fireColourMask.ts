import type { ManualQuad } from "./quadMask";
import type { MatchImage } from "./quadMatch";
type Colour=[number,number,number];
/** Palette construction follows fire-detection/core/src/presence.rs. */
export function firePalette(sheets:Uint8ClampedArray[]):Colour[] {
  const seen=new Set<number>(),colours:Colour[]=[];
  for(const sheet of sheets)for(let i=0;i<sheet.length;i+=4)if(sheet[i+3]) {
    const key=(sheet[i]<<16)|(sheet[i+1]<<8)|sheet[i+2];
    if(!seen.has(key)){seen.add(key);colours.push([sheet[i],sheet[i+1],sheet[i+2]]);}
  }
  return colours;
}
/** D24 palette neighbourhood from presence.rs; the brightness-independent G/R
 * tolerance0.06 and minimum red25 come from overlay.rs. Neutral dark grey is
 * deliberately not accepted by the gain branch (otherwise smoke becomes fire).
 */
export function isFirePaletteColour(r:number,g:number,b:number,palette:Colour[]):boolean {
  if(r<25)return false;
  for(const [R,G,B] of palette) {
    if((r-R)**2+(g-G)**2+(b-B)**2<=24**2)return true;
    const gain=r/Math.max(R,1);
    const warm=b<=r*0.6&&B<=R*0.6&&G>=R*0.3;
    // Keep dimmed yellow-white flame cores, but exclude dark neutral smoke.
    const brightCore=r>=130&&g>=r*0.8&&G>=R*0.8&&B>=R*0.6;
    if((warm||brightCore)&&gain>=0.25&&gain<=1.25&&Math.abs(g/r-G/Math.max(R,1))<=0.06&&Math.abs(b/r-B/Math.max(R,1))<=0.10)return true;
  }
  return false;
}
/** Candidate-independent segmentation, before trying any sheet/row/orientation. */
export function buildFireColourMask(image:MatchImage,sheets:[Uint8ClampedArray,Uint8ClampedArray],quad:ManualQuad):Uint8Array {
  const palette=firePalette(sheets),mask=new Uint8Array(image.width*image.height),cache=new Map<number,boolean>();
  const xs=quad.map(p=>p[0]),ys=quad.map(p=>p[1]);
  const dx=(Math.max(...xs)-Math.min(...xs))*0.2,dy=(Math.max(...ys)-Math.min(...ys))*0.2;
  const left=Math.max(0,Math.floor((Math.min(...xs)-dx)*image.width)),right=Math.min(image.width,Math.ceil((Math.max(...xs)+dx)*image.width));
  const top=Math.max(0,Math.floor((Math.min(...ys)-dy)*image.height)),bottom=Math.min(image.height,Math.ceil((Math.max(...ys)+dy)*image.height));
  for(let y=top;y<bottom;y++)for(let x=left;x<right;x++) {
    const i=y*image.width+x,r=image.data[i*4],g=image.data[i*4+1],b=image.data[i*4+2],key=(r<<16)|(g<<8)|b;
    let hit=cache.get(key);if(hit===undefined){hit=isFirePaletteColour(r,g,b,palette);cache.set(key,hit);}
    mask[i]=Number(hit);
  }
  return mask;
}
