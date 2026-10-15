import { useEffect, useRef, useState } from "react";
import fire0 from "../assets/fire_0.png";
import fire1 from "../assets/fire_1.png";
import { matchFireQuad, type MatchImage, type QuadCandidate, type QuadMatch } from "../lib/quadMatch";
import type { ManualQuad } from "../lib/quadMask";
import { Icon } from "./Icon";

interface Props { frameUrl:string; quad:ManualQuad; disabled:boolean; onPreview:(candidate:QuadCandidate)=>void }
function readImage(src:string):Promise<MatchImage> {
  return new Promise((resolve,reject)=> {
    const image=new Image();
    image.onload=()=> {
      try {
        const canvas=document.createElement("canvas");canvas.width=image.naturalWidth;canvas.height=image.naturalHeight;
        const ctx=canvas.getContext("2d",{willReadFrequently:true});if(!ctx)throw new Error("Canvas unavailable");
        ctx.drawImage(image,0,0);
        resolve({width:canvas.width,height:canvas.height,data:ctx.getImageData(0,0,canvas.width,canvas.height).data});
      }catch(e){reject(e);}
    };
    image.onerror=()=>reject(new Error("Could not load matching image"));image.src=src;
  });
}
let atlas:Promise<[Uint8ClampedArray,Uint8ClampedArray]>|null=null;
function readAtlas() {
  if(!atlas)atlas=Promise.all([readImage(fire0),readImage(fire1)]).then(images=> {
    if(images.some(i=>i.width!==16||i.height!==512))throw new Error("Invalid fire texture sheets");
    return [images[0].data,images[1].data] as [Uint8ClampedArray,Uint8ClampedArray];
  }).catch(e=>{atlas=null;throw e;});
  return atlas;
}
const identity=(frameUrl:string,quad:ManualQuad)=>JSON.stringify([frameUrl,quad]);
const name=(c:QuadCandidate)=>`sheet ${c.sheet} · row ${c.row} · ${c.mirrored?"mirrored":"normal"}`;

export function QuadMatchControls({frameUrl,quad,disabled,onPreview}:Props) {
  const key=identity(frameUrl,quad),latest=useRef({key,disabled,onPreview});latest.current={key,disabled,onPreview};
  const token=useRef(0);
  const [busy,setBusy]=useState<string|null>(null);
  const [result,setResult]=useState<{key:string;data?:QuadMatch;error?:string;selected?:QuadCandidate}|null>(null);
  useEffect(()=>()=>{token.current++;},[]);
  const current=result?.key===key?result:null;
  async function find() {
    const mine=++token.current,start=key;
    setBusy(start);setResult(null);
    try {
      // Read the original JPEG, never the displayed overlay or a screenshot.
      const [image,sheets]=await Promise.all([readImage(frameUrl),readAtlas()]);
      await new Promise(resolve=>window.setTimeout(resolve,0));
      if(mine!==token.current||latest.current.key!==start||latest.current.disabled)return;
      const data=matchFireQuad(image,sheets,quad);
      if(mine!==token.current||latest.current.key!==start||latest.current.disabled)return;
      const best=data.status==="clear"?data.candidates[0]:undefined;
      setResult({key:best?identity(frameUrl,best.quad):start,data,selected:best});
      if(best)latest.current.onPreview(best);
    }catch(e){if(mine===token.current&&latest.current.key===start)setResult({key:start,error:String(e)});}
    finally{if(mine===token.current)setBusy(null);}
  }
  function preview(candidate:QuadCandidate) {
    if(!current?.data||disabled)return;
    setResult({...current,key:identity(frameUrl,candidate.quad),selected:candidate});
    onPreview(candidate);
  }
  return <>
    <button disabled={disabled||busy===key} onClick={()=>void find()} title="Compare the outlined fire with all 128 sheet/row/mirror combinations. Refines position/size slightly; previews only, never saves a label.">
      <Icon name="zoomIn" /> {busy===key?"Matching…":"Find best match"}
    </button>
    {current?.error&&<span className="warn" role="alert">Match failed: {current.error}</span>}
    {current?.data&&<>
      <span className="hud-hint" role="status">{current.selected?`Preview: ${name(current.selected)} · confirm separately`
        :current.data.status==="insufficient"?"Too few visible pixels—adjust the quad or use a larger fire"
        :current.data.status==="weak"?"No clear texture match—adjust the quad; closest candidates below"
        :"Close matches—pick a candidate to preview"}</span>
      {current.data.candidates.slice(0,3).map((c,i)=><button key={`${c.sheet}-${c.row}-${c.mirrored}`}
        disabled={disabled} aria-pressed={current.selected===c} onClick={()=>preview(c)}
        title={`Preview ${name(c)}. Similarity is a ranking score, not a probability.`}>
        {i+1}: S{c.sheet} R{c.row}{c.mirrored?" M":" N"} · {c.score.toFixed(2)}
      </button>)}
    </>}
  </>;
}
