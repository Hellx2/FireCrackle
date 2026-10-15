import {projectQuad,validQuad,type ManualQuad} from "./quadMask";

export interface QuadOptimum {quad:ManualQuad;score:number;evaluations:number}
type Vertex={point:number[];quad:ManualQuad;score:number};
const UNIT=[0,0,1,0,1,1,0,1];
const bound=(x:number)=>Math.max(-0.1,Math.min(1.1,x));

/** Bounded Nelder–Mead fit of all eight corner coordinates. The envelope is
 * the original quad expanded to 120% about its centre, in its own plane.
 * This is a numerical local optimum, not a guarantee of the global optimum.
 */
export function optimizeQuad(origin:ManualQuad,evidence:(quad:ManualQuad)=>number,maxEvaluations=220,seed?:ManualQuad):QuadOptimum {
  let evaluations=0;
  function evaluate(point:number[]):Vertex {
    point=point.map(bound);
    const quad=point===UNIT?origin:[0,1,2,3].map(i=>projectQuad(origin,point[i*2],point[i*2+1])) as ManualQuad;
    evaluations++;
    const value=validQuad(quad)?evidence(quad):Number.NEGATIVE_INFINITY;
    return {point,quad,score:Number.isFinite(value)?value:Number.NEGATIVE_INFINITY};
  }
  const start=seed?seed.flatMap(([x,y])=>{
    let u=.5,v=.5;
    for(let k=0;k<12;k++){
      const p=projectQuad(origin,u,v),a=projectQuad(origin,u+.0001,v),b=projectQuad(origin,u,v+.0001);
      const ax=(a[0]-p[0])/.0001,ay=(a[1]-p[1])/.0001,bx=(b[0]-p[0])/.0001,by=(b[1]-p[1])/.0001,det=ax*by-ay*bx;
      if(Math.abs(det)<1e-14)break;
      u-=((p[0]-x)*by-(p[1]-y)*bx)/det;v-=(ax*(p[1]-y)-ay*(p[0]-x))/det;
    }
    return [u,v];
  }):UNIT;
  const initial=evaluate(UNIT),seeded=seed?evaluate(start):initial,simplex=[seeded];
  for(let axis=0;axis<8;axis++){
    const point=start.slice();point[axis]+=point[axis]>.9?-.06:.06;
    simplex.push(evaluate(point));
  }
  while(evaluations+10<maxEvaluations){
    simplex.sort((a,b)=>b.score-a.score);
    const best=simplex[0],worst=simplex[8],secondWorst=simplex[7];
    const spread=Math.max(...simplex.flatMap(v=>v.point.map((x,i)=>Math.abs(x-best.point[i]))));
    if(spread<0.002)break;
    const centre=UNIT.map((_,i)=>simplex.slice(0,8).reduce((sum,v)=>sum+v.point[i],0)/8);
    const reflected=evaluate(centre.map((x,i)=>2*x-worst.point[i]));
    if(reflected.score>best.score){
      const expanded=evaluate(centre.map((x,i)=>x+2*(reflected.point[i]-x)));
      simplex[8]=expanded.score>reflected.score?expanded:reflected;
    }else if(reflected.score>secondWorst.score){
      simplex[8]=reflected;
    }else{
      const outside=reflected.score>worst.score;
      const contracted=evaluate(centre.map((x,i)=>x+0.5*((outside?reflected.point[i]:worst.point[i])-x)));
      if(contracted.score>(outside?reflected.score:worst.score))simplex[8]=contracted;
      else for(let i=1;i<9;i++)simplex[i]=evaluate(best.point.map((x,axis)=>x+0.5*(simplex[i].point[axis]-x)));
    }
  }
  simplex.sort((a,b)=>b.score-a.score);
  // Equal or worse fits never move a user's outline gratuitously.
  const best=simplex[0].score>initial.score?simplex[0]:initial;
  return {quad:best===initial?origin:best.quad,score:best.score,evaluations};
}
