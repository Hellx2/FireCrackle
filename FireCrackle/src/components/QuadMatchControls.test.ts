import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
import type { ManualQuad } from "../lib/quadMask";
import type { QuadCandidate } from "../lib/quadMatch";

const hooks=vi.hoisted(()=>({values:[] as unknown[],cursor:0,cleanup:undefined as undefined|(()=>void)}));
vi.mock("react",async original=>({...(await original<typeof import("react")>()),
  useRef:(initial:unknown)=> {const i=hooks.cursor++;return hooks.values[i]??=( {current:initial} );},
  useState:(initial:unknown)=> {const i=hooks.cursor++;if(!(i in hooks.values))hooks.values[i]=initial;return [hooks.values[i],(value:unknown)=>{hooks.values[i]=value;}];},
  useEffect:(setup:()=>void|(()=>void))=> {if(!hooks.cleanup)hooks.cleanup=setup()||undefined;},
}));
vi.mock("../lib/quadMatch",()=>({matchFireQuad:vi.fn()}));
import { matchFireQuad } from "../lib/quadMatch";
import { QuadMatchControls } from "./QuadMatchControls";

const pending=new Map<string,{onload:(()=>void)|null}>();
const q:ManualQuad=[[0,0],[1,0],[1,1],[0,1]];
const best={sheet:1 as const,row:12,mirrored:true,score:0.95,quad:q};
function render(frameUrl:string,onPreview:(candidate:QuadCandidate)=>void,disabled=false) {
  hooks.cursor=0;return QuadMatchControls({frameUrl,quad:q,disabled,onPreview});
}
function buttons(node:unknown):ReactElement<Record<string,unknown>>[] {
  if(Array.isArray(node))return node.flatMap(buttons);
  if(!node||typeof node!=="object"||!("props" in node))return [];
  const el=node as ReactElement<Record<string,unknown>>;
  return [...(el.type==="button"?[el]:[]),...buttons(el.props.children)];
}
async function settle(){for(let i=0;i<20;i++)await Promise.resolve();}
beforeEach(()=> {
  hooks.values=[];hooks.cursor=0;hooks.cleanup=undefined;pending.clear();vi.clearAllMocks();
  class ImageMock {
    naturalWidth=16;naturalHeight=512;onload:(()=>void)|null=null;
    set src(src:string) {if(src.startsWith("frame")){this.naturalWidth=160;this.naturalHeight=100;pending.set(src,this);}else queueMicrotask(()=>this.onload?.());}
  }
  vi.stubGlobal("Image",ImageMock);
  vi.stubGlobal("document",{createElement:()=>({width:0,height:0,getContext:()=>({drawImage:()=>{},getImageData:(_x:number,_y:number,w:number,h:number)=>({data:new Uint8ClampedArray(w*h*4)})})})});
  vi.stubGlobal("window",{setTimeout:(cb:()=>void)=>{queueMicrotask(cb);return 0;}});
  vi.mocked(matchFireQuad).mockReturnValue({status:"clear",candidates:[best],margin:0.1,searched:128});
});
afterEach(()=>{hooks.cleanup?.();vi.unstubAllGlobals();});

it("previews a clear match only after original image decoding completes",async()=> {
  const preview=vi.fn();(buttons(render("frame-a",preview))[0].props.onClick as ()=>void)();
  expect(preview).not.toHaveBeenCalled();
  pending.get("frame-a")!.onload?.();await settle();
  expect(preview).toHaveBeenCalledExactlyOnceWith(best);
});
it("discards a result if the user navigates to another frame while it loads",async()=> {
  const preview=vi.fn();(buttons(render("frame-a",preview))[0].props.onClick as ()=>void)();
  render("frame-b",preview);pending.get("frame-a")!.onload?.();await settle();
  expect(matchFireQuad).not.toHaveBeenCalled();expect(preview).not.toHaveBeenCalled();
});
it("leaves ambiguous matches untouched until a candidate is explicitly clicked",async()=> {
  vi.mocked(matchFireQuad).mockReturnValue({status:"ambiguous",candidates:[best],margin:0.01,searched:128});
  const preview=vi.fn();(buttons(render("frame-a",preview))[0].props.onClick as ()=>void)();
  pending.get("frame-a")!.onload?.();await settle();expect(preview).not.toHaveBeenCalled();
  const candidates=buttons(render("frame-a",preview));
  (candidates[1].props.onClick as ()=>void)();expect(preview).toHaveBeenCalledExactlyOnceWith(best);
});
it("discards an in-flight result if the source frame becomes unavailable",async()=> {
  const preview=vi.fn();(buttons(render("frame-a",preview))[0].props.onClick as ()=>void)();
  render("frame-a",preview,true);pending.get("frame-a")!.onload?.();await settle();
  expect(preview).not.toHaveBeenCalled();expect(matchFireQuad).not.toHaveBeenCalled();
});
