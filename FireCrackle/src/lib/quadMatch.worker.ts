import {matchFireQuad,type MatchImage,type QuadMatch} from "./quadMatch";
import type {ManualQuad} from "./quadMask";
import type {EdgeFilter} from "./atlasFilter";

type Request={image:MatchImage;sheets:[Uint8ClampedArray,Uint8ClampedArray];quad:ManualQuad;options:{refineGeometry?:boolean;filter?:EdgeFilter}};
const scope=self as unknown as {onmessage:(event:MessageEvent<Request>)=>void;postMessage:(response:{result?:QuadMatch;error?:string})=>void};
scope.onmessage=({data})=>{
  try{scope.postMessage({result:matchFireQuad(data.image,data.sheets,data.quad,data.options)});}
  catch(error){scope.postMessage({error:String(error)});}
};
