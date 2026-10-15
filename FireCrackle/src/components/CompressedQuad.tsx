import { useEffect, useMemo, useState } from "react";
import { renderCompressedQuad, type CompressionTexel } from "../lib/compressionPreview";

interface Props {
  frameUrl:string; texels:CompressionTexel[]; width:number; height:number;
  displayWidth:number; displayHeight:number; opacity:number; quality:number;
}

/** Debounce edits, cancel stale work, and never show a previous frame/settings result. */
export function CompressedQuad(props:Props) {
  const {frameUrl,texels,width,height,displayWidth,displayHeight,opacity,quality}=props;
  const request=useMemo(()=>({frameUrl,texels,width,height,quality}),[frameUrl,texels,width,height,quality]);
  const [result,setResult]=useState<{request:typeof request;url?:string;error?:string}|null>(null);
  useEffect(()=> {
    let cancelled=false, objectUrl:string|undefined;
    const timer=window.setTimeout(()=> {
      renderCompressedQuad(frameUrl,texels,width,height,quality).then(blob=> {
        if(cancelled)return;
        objectUrl=URL.createObjectURL(blob);setResult({request,url:objectUrl});
      }).catch(e=> {if(!cancelled)setResult({request,error:String(e)});});
    },120);
    return ()=> {cancelled=true;window.clearTimeout(timer);if(objectUrl)URL.revokeObjectURL(objectUrl);};
  },[request]);
  const current=result?.request===request?result:null;
  return current?.url
    ? <image href={current.url} width={displayWidth} height={displayHeight} opacity={opacity} preserveAspectRatio="none" style={{imageRendering:"pixelated"}} />
    : <text x="12" y="24" fill="white" stroke="black" paintOrder="stroke" strokeWidth="2" role={current?.error?"alert":"status"}>
      {current?.error?`Compression preview failed: ${current.error}`:"Rendering compression preview…"}
    </text>;
}
