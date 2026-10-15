import {expect,it} from "vitest";
import {optimizeQuad} from "./quadOptimizer";
import {validQuad,type ManualQuad} from "./quadMask";

const origin:ManualQuad=[[0,0],[1,0],[1,1],[0,1]];
it("fits independent perspective corners rather than a size/position grid",()=>{
  const target:ManualQuad=[[.07,.03],[.94,.08],[1.04,.96],[-.03,1.03]];
  const score=(q:ManualQuad)=>-q.reduce((sum,p,i)=>sum+(p[0]-target[i][0])**2+(p[1]-target[i][1])**2,0);
  const fit=optimizeQuad(origin,score,650);
  expect(fit.score).toBeGreaterThan(score(origin)*.01);
  expect(validQuad(fit.quad)).toBe(true);
});
it("never evaluates a folded quad or leaves the 120% envelope",()=>{
  const seen:ManualQuad[]=[];
  optimizeQuad(origin,q=>{seen.push(q);return q.reduce((s,p)=>s+p[0]-p[1],0);},250);
  expect(seen.length).toBeGreaterThan(9);
  for(const q of seen){expect(validQuad(q)).toBe(true);for(const p of q)for(const x of p){expect(x).toBeGreaterThanOrEqual(-.100001);expect(x).toBeLessThanOrEqual(1.100001);}}
});
it("leaves equal-evidence and empty-evidence outlines untouched",()=>{
  expect(optimizeQuad(origin,()=>0).quad).toBe(origin);
  expect(optimizeQuad(origin,()=>Number.NaN).quad).toBe(origin);
});
