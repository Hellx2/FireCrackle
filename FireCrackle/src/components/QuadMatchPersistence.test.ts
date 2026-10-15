import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
import { PREFERENCES_KEY } from "../lib/preferences";
const hooks=vi.hoisted(()=>({states:[] as unknown[],cursor:0,effects:[] as Array<()=>unknown>}));
vi.mock("react",async original=>({...await original<typeof import("react")>(),
  useRef:(current:unknown)=>({current}),
  useState:(initial:unknown)=>{const i=hooks.cursor++;if(!(i in hooks.states))hooks.states[i]=initial;return [hooks.states[i],(next:unknown)=>{hooks.states[i]=next;}];},
  useEffect:(effect:()=>unknown)=>{hooks.effects.push(effect);},
}));
import { QuadMatchControls } from "./QuadMatchControls";
type Element=ReactElement<Record<string,any>>;
function find(node:Element,label:string):Element|undefined {
  if(node.props["aria-label"]===label)return node;
  for(const child of [node.props.children].flat(Infinity))if(child&&typeof child==="object"&&"props" in child){const found=find(child,label);if(found)return found;}
}
function render() {hooks.cursor=0;hooks.effects=[];return QuadMatchControls({frameUrl:"frame-a",quad:[[0,0],[1,0],[1,1],[0,1]],disabled:false,onPreview:vi.fn()});}
beforeEach(()=>{hooks.states=[];});
afterEach(()=>vi.unstubAllGlobals());
it("restores explicitly disabled match options and saves toggles across remounts",()=>{
  let saved=JSON.stringify({matchAdjustQuad:false,matchOnlyFiltered:false,atlasFilter:{left:3,right:7}});
  vi.stubGlobal("localStorage",{getItem:(key:string)=>key===PREFERENCES_KEY?saved:null,setItem:(key:string,value:string)=>{if(key===PREFERENCES_KEY)saved=value;}});
  let root=render();
  expect(find(root,"Adjust quad while matching")!.props.checked).toBe(false);
  expect(find(root,"Only filtered sprites")!.props.checked).toBe(false);
  find(root,"Adjust quad while matching")!.props.onChange({target:{checked:true}});
  root=render();hooks.effects[hooks.effects.length-1]();
  expect(JSON.parse(saved)).toMatchObject({matchAdjustQuad:true,matchOnlyFiltered:false,atlasFilter:{left:3,right:7}});
  hooks.states=[];root=render();
  expect(find(root,"Adjust quad while matching")!.props.checked).toBe(true);
  expect(find(root,"Only filtered sprites")!.props.checked).toBe(false);
});
