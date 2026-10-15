import { projectQuad, validQuad, type ManualQuad } from "./quadMask";
import type { PreviewSprite } from "./firePreview";
import { buildFireColourMask } from "./fireColourMask";

export interface MatchImage { width:number; height:number; data:Uint8ClampedArray }
export interface QuadCandidate extends PreviewSprite { score:number; quad:ManualQuad; support?:number; maskSamples?:number; regions?:number }
export interface QuadMatch {
  status:"clear"|"ambiguous"|"weak"|"insufficient";
  candidates:QuadCandidate[]; margin:number; searched:number;
}
interface Samples {rgb:Float64Array;fire:Uint8Array;valid:Uint8Array;count:number}
interface Template extends PreviewSprite {rgb:Float64Array;alpha:Uint8Array; bounds:[number,number,number,number]}

function sampleFrame(image:MatchImage,quad:ManualQuad,mask:Uint8Array):Samples {
  const rgb=new Float64Array(768),fire=new Uint8Array(256),valid=new Uint8Array(256);
  let count=0;
  for(let py=0;py<16;py++)for(let px=0;px<16;px++) {
    const i=py*16+px,[u,v]=projectQuad(quad,(px+0.5)/16,(py+0.5)/16);
    const X=u*image.width-0.5,Y=v*image.height-0.5;
    if(X<0||Y<0||X>image.width-1||Y>image.height-1)continue;
    const x0=Math.floor(X),y0=Math.floor(Y),x1=Math.min(image.width-1,x0+1),y1=Math.min(image.height-1,y0+1),fx=X-x0,fy=Y-y0;
    for(let c=0;c<3;c++)rgb[i*3+c]=
      image.data[(y0*image.width+x0)*4+c]*(1-fx)*(1-fy)+image.data[(y0*image.width+x1)*4+c]*fx*(1-fy)+
      image.data[(y1*image.width+x0)*4+c]*(1-fx)*fy+image.data[(y1*image.width+x1)*4+c]*fx*fy;
    fire[i]=Number(mask[y0*image.width+x0]*(1-fx)*(1-fy)+mask[y0*image.width+x1]*fx*(1-fy)+mask[y1*image.width+x0]*(1-fx)*fy+mask[y1*image.width+x1]*fx*fy>=0.5);valid[i]=1;count++;
  }
  return {rgb,fire,valid,count};
}

function templates(sheets:[Uint8ClampedArray,Uint8ClampedArray]):Template[] {
  const result:Template[]=[];
  for(const sheet of [0,1] as const)for(let row=0;row<32;row++)for(const mirrored of [false,true]) {
    const rgb=new Float64Array(768),alpha=new Uint8Array(256);
    for(let py=0;py<16;py++)for(let px=0;px<16;px++) {
      const i=py*16+px,source=((row*16+py)*16+(mirrored?15-px:px))*4;
      for(let c=0;c<3;c++)rgb[i*3+c]=sheets[sheet][source+c];
      alpha[i]=sheets[sheet][source+3];
    }
    const opaque=Array.from(alpha.keys()).filter(i=>alpha[i]>127);
    const bounds:[number,number,number,number]=opaque.length?[
      Math.min(...opaque.map(i=>i%16))/16,Math.min(...opaque.map(i=>Math.floor(i/16)))/16,
      (Math.max(...opaque.map(i=>i%16))+1)/16,(Math.max(...opaque.map(i=>Math.floor(i/16)))+1)/16,
    ]:[0,0,1,1];
    result.push({sheet,row,mirrored,rgb,alpha,bounds});
  }
  return result;
}

/** Positive-only evidence from the candidate-independent colour mask.
 * Each of the fixed 256 positions can add a vote; unknown pixels, transparency
 * and colour mismatches add zero. Never divide by the observed mask size:
 * revealing another sample must not dilute evidence already collected.
 */
function similarity(observed:Samples,model:Template):{score:number;support:number;maskSamples:number;regions:number} {
  let votes=0,maskSamples=0,support=0;
  const regions=new Set<number>();
  for(let i=0;i<256;i++) {
    if(!observed.valid[i]||!observed.fire[i])continue;
    maskSamples++;
    if(model.alpha[i]<=127)continue;
    const a=i*3,R=Math.max(model.rgb[a],1),r=Math.max(observed.rgb[a],1);
    const dg=Math.abs(observed.rgb[a+1]/r-model.rgb[a+1]/R);
    const db=Math.abs(observed.rgb[a+2]/r-model.rgb[a+2]/R);
    if(dg>0.06||db>0.10)continue;
    // Close colour agreement adds more support, without any negative residual.
    votes+=1-0.5*(dg/0.06+db/0.10);
    support++;regions.add(Math.floor((i%16)/4)+4*Math.floor(i/64));
  }
  return {score:votes/256,support,maskSamples,regions:regions.size};
}

/** Search128 appearances and small position/scale corrections inside the supplied outline.
 * No clock prediction or UI overlay is an input; only the original frame pixels.
 */
export function matchFireQuad(image:MatchImage,sheets:[Uint8ClampedArray,Uint8ClampedArray],quad:ManualQuad):QuadMatch {
  return match(image,sheets,quad,false);
}

/** Oracle-box evaluation adapter: labels bound visible opacity, not the full
 * texture. Expand each candidate using ITS OWN alpha hull, never the truth row.
 * A rectangle cannot recover perspective or hidden texture from a clipped box.
 */
export function matchFireBox(image:MatchImage,sheets:[Uint8ClampedArray,Uint8ClampedArray],box:ManualQuad,options={stretch:true}):QuadMatch {
  return match(image,sheets,box,true,options.stretch);
}

function match(image:MatchImage,sheets:[Uint8ClampedArray,Uint8ClampedArray],quad:ManualQuad,opaqueBox:boolean,stretch=true):QuadMatch {
  const insufficient:QuadMatch={status:"insufficient",candidates:[],margin:0,searched:0};
  if(!validQuad(quad)||image.width<=0||image.height<=0||image.data.length!==image.width*image.height*4||sheets.some(s=>s.length!==16*512*4))return insufficient;
  const w=Math.hypot((quad[1][0]-quad[0][0])*image.width,(quad[1][1]-quad[0][1])*image.height);
  const h=Math.hypot((quad[3][0]-quad[0][0])*image.width,(quad[3][1]-quad[0][1])*image.height);
  if(w*h<36||Math.min(w,h)<4)return insufficient;
  const mask=buildFireColourMask(image,sheets,quad);
  const models=templates(sheets);
  const best:QuadCandidate[]=models.map(({sheet,row,mirrored})=>({sheet,row,mirrored,score:0,quad}));
  let enoughVisible=false;
  const scales=stretch?[1,0.8,1.2].flatMap(x=>[1,0.8,1.2].map(y=>[x,y])):[[1,1],[0.92,0.92],[1.08,1.08]];
  for(const [scaleX,scaleY] of scales)for(const dx of [0,-0.05,0.05])for(const dy of [0,-0.05,0.05]) {
    const fit=[[0,0],[1,0],[1,1],[0,1]].map(([u,v])=>projectQuad(quad,(u-0.5)*scaleX+0.5+dx,(v-0.5)*scaleY+0.5+dy)) as ManualQuad;
    if(!validQuad(fit))continue;
    const shared=opaqueBox?null:sampleFrame(image,fit,mask);
    const cache=new Map<string,{observed:Samples;expanded:ManualQuad}>();
    for(let i=0;i<models.length;i++) {
      let observed=shared,expanded=fit;
      if(opaqueBox){
        const [x0,y0,x1,y1]=models[i].bounds,key=models[i].bounds.join(",");
        let entry=cache.get(key);
        if(!entry){
          expanded=[[0,0],[1,0],[1,1],[0,1]].map(([u,v])=>projectQuad(fit,(u-x0)/(x1-x0),(v-y0)/(y1-y0))) as ManualQuad;
          entry={observed:sampleFrame(image,expanded,mask),expanded};cache.set(key,entry);
        }
        ({observed,expanded}=entry);
      }
      if(!observed||observed.count<64)continue;
      enoughVisible=true;
      const evidence=similarity(observed,models[i]);
      if(evidence.score>best[i].score)best[i]={...best[i],...evidence,quad:expanded};
    }
  }
  if(!enoughVisible)return insufficient;
  best.sort((a,b)=>b.score-a.score);
  const margin=best[0].score-best[1].score;
  const status=best[0].score===0?"weak":(best[0].support??0)<32||(best[0].regions??0)<3?"insufficient":margin<0.03?"ambiguous":"clear";
  return {status,candidates:best.slice(0,5),margin,searched:128};
}
