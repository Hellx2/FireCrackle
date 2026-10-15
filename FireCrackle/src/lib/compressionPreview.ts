import type { ManualQuad } from "./quadMask";

export interface CompressionSettings { height: number; quality: number }
export interface CompressionTexel { points: Array<[number, number]>; colour: string; alpha: number }

/** Delivery coordinates only: CSS size, devicePixelRatio, zoom and pan never enter here. */
export function compressionGeometry(width: number, height: number, targetHeight: number, quad: ManualQuad) {
  // The decoder may supply a reduced preview. Its aspect ratio locates the quad,
  // but only the selected delivery resolution determines the fire's raster size.
  const h = Math.max(1, Math.round(targetHeight));
  const w = Math.max(1, Math.round(width * h / height));
  return { width:w, height:h,
    quadWidth:(Math.max(...quad.map(p=>p[0]))-Math.min(...quad.map(p=>p[0])))*w,
    quadHeight:(Math.max(...quad.map(p=>p[1]))-Math.min(...quad.map(p=>p[1])))*h };
}

/** SVG is rasterized at delivery size BEFORE any lossy encoding or editor magnification. */
export function compressionSvg(texels: CompressionTexel[], width: number, height: number) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" overflow="hidden"><g shape-rendering="crispEdges">${texels.map(p =>
    `<polygon points="${p.points.map(([x,y])=>`${x*width},${y*height}`).join(" ")}" fill="${p.colour}" fill-opacity="${p.alpha}"/>`).join("")}</g></svg>`;
}

/** Approximate 4:2:0 colour loss: share chroma across each 2x2 delivery-pixel cell. */
export function subsampleChroma420(data: Uint8ClampedArray, width: number, height: number) {
  for(let y=0;y<height;y+=2) for(let x=0;x<width;x+=2) {
    let cb=0,cr=0,n=0;
    for(let dy=0;dy<2 && y+dy<height;dy++) for(let dx=0;dx<2 && x+dx<width;dx++) {
      const i=((y+dy)*width+x+dx)*4, r=data[i],g=data[i+1],b=data[i+2];
      cb+=-0.168736*r-0.331264*g+0.5*b;
      cr+=0.5*r-0.418688*g-0.081312*b; n++;
    }
    cb/=n;cr/=n;
    for(let dy=0;dy<2 && y+dy<height;dy++) for(let dx=0;dx<2 && x+dx<width;dx++) {
      const i=((y+dy)*width+x+dx)*4, luma=0.299*data[i]+0.587*data[i+1]+0.114*data[i+2];
      data[i]=luma+1.402*cr; data[i+1]=luma-0.344136*cb-0.714136*cr; data[i+2]=luma+1.772*cb;
    }
  }
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve,reject)=> {
    const image=new Image(); image.onload=()=>resolve(image); image.onerror=()=>reject(new Error("Could not load compression source")); image.src=src;
  });
}

/** Still-frame stress preview, not a calibrated YouTube encoder or temporal codec model.
 * Scene context is used offscreen for colour/block alignment only. Restore the fire's
 * alpha after decoding so the returned PNG overlays ONLY the mask, never the video.
 * The caller applies comparison opacity once to this transparent result.
 */
export async function renderCompressedQuad(frameUrl: string, texels: CompressionTexel[], width: number, height: number, quality: number): Promise<Blob> {
  const spriteUrl=URL.createObjectURL(new Blob([compressionSvg(texels,width,height)],{type:"image/svg+xml"}));
  let encodedUrl:string|undefined;
  try {
    const [frame,sprite]=await Promise.all([loadImage(frameUrl),loadImage(spriteUrl)]);
    const canvas=document.createElement("canvas");canvas.width=width;canvas.height=height;
    const ctx=canvas.getContext("2d",{willReadFrequently:true});if(!ctx) throw new Error("Canvas unavailable");
    ctx.drawImage(frame,0,0,width,height);
    ctx.drawImage(sprite,0,0);
    const pixels=ctx.getImageData(0,0,width,height);
    subsampleChroma420(pixels.data,width,height);ctx.putImageData(pixels,0,0);
    const encoded=await new Promise<Blob>((resolve,reject)=>canvas.toBlob(blob=> {
      if(blob?.type==="image/jpeg") resolve(blob); else reject(new Error("JPEG compression unavailable"));
    },"image/jpeg",quality));
    encodedUrl=URL.createObjectURL(encoded);
    const damaged=await loadImage(encodedUrl);
    ctx.clearRect(0,0,width,height);
    ctx.drawImage(damaged,0,0);
    // Remove every background pixel, including holes inside the fire silhouette.
    ctx.globalCompositeOperation="destination-in";
    ctx.drawImage(sprite,0,0);
    ctx.globalCompositeOperation="source-over";
    return await new Promise<Blob>((resolve,reject)=>canvas.toBlob(blob=> {
      if(blob?.type==="image/png") resolve(blob); else reject(new Error("Transparent compression preview unavailable"));
    },"image/png"));
  } finally {
    URL.revokeObjectURL(spriteUrl);
    if(encodedUrl)URL.revokeObjectURL(encodedUrl);
  }
}
