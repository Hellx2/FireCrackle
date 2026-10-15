import { projectQuad, validQuad, type ManualQuad } from "./quadMask";
import type { PreviewSprite } from "./firePreview";

export interface MatchImage { width:number; height:number; data:Uint8ClampedArray }
export interface QuadCandidate extends PreviewSprite { score:number; quad:ManualQuad }
export interface QuadMatch {
  status:"clear"|"ambiguous"|"weak"|"insufficient";
  candidates:QuadCandidate[]; margin:number; searched:number;
}
interface Samples {rgb:Float64Array;y:Float64Array;fire:Uint8Array;valid:Uint8Array;count:number}
interface Template extends PreviewSprite {rgb:Float64Array;y:Float64Array;alpha:Uint8Array}
const luminance=(r:number,g:number,b:number)=>0.299*r+0.587*g+0.114*b;
const fireColour=(r:number,g:number,b:number)=>r>25 && ((r>b+12 && g>b*1.05 && g>r*0.25) || Math.min(r,g,b)>185);

function sampleFrame(image:MatchImage,quad:ManualQuad):Samples {
  const rgb=new Float64Array(768),y=new Float64Array(256),fire=new Uint8Array(256),valid=new Uint8Array(256);
  let count=0;
  for(let py=0;py<16;py++)for(let px=0;px<16;px++) {
    const i=py*16+px,[u,v]=projectQuad(quad,(px+0.5)/16,(py+0.5)/16);
    const X=u*image.width-0.5,Y=v*image.height-0.5;
    if(X<0||Y<0||X>image.width-1||Y>image.height-1)continue;
    const x0=Math.floor(X),y0=Math.floor(Y),x1=Math.min(image.width-1,x0+1),y1=Math.min(image.height-1,y0+1),fx=X-x0,fy=Y-y0;
    for(let c=0;c<3;c++)rgb[i*3+c]=
      image.data[(y0*image.width+x0)*4+c]*(1-fx)*(1-fy)+image.data[(y0*image.width+x1)*4+c]*fx*(1-fy)+
      image.data[(y1*image.width+x0)*4+c]*(1-fx)*fy+image.data[(y1*image.width+x1)*4+c]*fx*fy;
    y[i]=luminance(rgb[i*3],rgb[i*3+1],rgb[i*3+2]);
    fire[i]=Number(fireColour(rgb[i*3],rgb[i*3+1],rgb[i*3+2]));valid[i]=1;count++;
  }
  return {rgb,y,fire,valid,count};
}

function templates(sheets:[Uint8ClampedArray,Uint8ClampedArray]):Template[] {
  const result:Template[]=[];
  for(const sheet of [0,1] as const)for(let row=0;row<32;row++)for(const mirrored of [false,true]) {
    const rgb=new Float64Array(768),y=new Float64Array(256),alpha=new Uint8Array(256);
    for(let py=0;py<16;py++)for(let px=0;px<16;px++) {
      const i=py*16+px,source=((row*16+py)*16+(mirrored?15-px:px))*4;
      for(let c=0;c<3;c++)rgb[i*3+c]=sheets[sheet][source+c];
      y[i]=luminance(rgb[i*3],rgb[i*3+1],rgb[i*3+2]);alpha[i]=sheets[sheet][source+3];
    }
    result.push({sheet,row,mirrored,rgb,y,alpha});
  }
  return result;
}

/** Brightness-normalized pattern correlation plus colour and silhouette evidence.
 * Scores are relative similarities, never probabilities or calibrated confidence.
 */
function similarity(observed:Samples,model:Template):number {
  let n=0,t=0,o=0,tt=0,oo=0,to=0,colour=0,intersection=0,union=0;
  for(let i=0;i<256;i++) {
    if(!observed.valid[i])continue;
    const opaque=model.alpha[i]>127;
    if(opaque||observed.fire[i])union++;
    if(opaque&&observed.fire[i])intersection++;
    if(!opaque)continue;
    const T=model.y[i],O=observed.y[i];
    n++;t+=T;o+=O;tt+=T*T;oo+=O*O;to+=T*O;
    const a=i*3,sumT=model.rgb[a]+model.rgb[a+1]+model.rgb[a+2]+1,sumO=observed.rgb[a]+observed.rgb[a+1]+observed.rgb[a+2]+1;
    let difference=0;
    for(let c=0;c<3;c++)difference+=Math.abs(model.rgb[a+c]/sumT-observed.rgb[a+c]/sumO);
    colour+=Math.max(0,1-difference*2);
  }
  if(n<32)return 0;
  const varT=tt-t*t/n,varO=oo-o*o/n;
  if(varT/n<25||varO/n<25)return 0;
  const correlation=Math.max(0,(to-t*o/n)/Math.sqrt(varT*varO));
  return Math.min(1,0.78*correlation+0.14*colour/n+0.08*(union?intersection/union:0));
}

/** Search128 appearances and small position/scale corrections inside the supplied outline.
 * No clock prediction or UI overlay is an input; only the original frame pixels.
 */
export function matchFireQuad(image:MatchImage,sheets:[Uint8ClampedArray,Uint8ClampedArray],quad:ManualQuad):QuadMatch {
  const insufficient:QuadMatch={status:"insufficient",candidates:[],margin:0,searched:0};
  if(!validQuad(quad)||image.width<=0||image.height<=0||image.data.length!==image.width*image.height*4||sheets.some(s=>s.length!==16*512*4))return insufficient;
  const w=Math.hypot((quad[1][0]-quad[0][0])*image.width,(quad[1][1]-quad[0][1])*image.height);
  const h=Math.hypot((quad[3][0]-quad[0][0])*image.width,(quad[3][1]-quad[0][1])*image.height);
  if(w*h<36||Math.min(w,h)<4)return insufficient;
  const models=templates(sheets);
  const best:QuadCandidate[]=models.map(({sheet,row,mirrored})=>({sheet,row,mirrored,score:0,quad}));
  let enoughVisible=false;
  for(const scale of [1,0.92,1.08])for(const dx of [0,-0.05,0.05])for(const dy of [0,-0.05,0.05]) {
    const fit=[[0,0],[1,0],[1,1],[0,1]].map(([u,v])=>projectQuad(quad,(u-0.5)*scale+0.5+dx,(v-0.5)*scale+0.5+dy)) as ManualQuad;
    if(!validQuad(fit))continue;
    const observed=sampleFrame(image,fit);
    if(observed.count<64)continue;
    enoughVisible=true;
    for(let i=0;i<models.length;i++) {
      const score=similarity(observed,models[i]);
      if(score>best[i].score)best[i]={...best[i],score,quad:fit};
    }
  }
  if(!enoughVisible)return insufficient;
  best.sort((a,b)=>b.score-a.score);
  const margin=best[0].score-best[1].score;
  return {status:best[0].score<0.78?"weak":margin<0.03?"ambiguous":"clear",candidates:best.slice(0,5),margin,searched:128};
}
