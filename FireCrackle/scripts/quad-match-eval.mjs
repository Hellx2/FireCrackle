// Runs the actual app matcher, with truth used only after prediction.
// node scripts/quad-match-eval.mjs OUT_FROM_PREPARE [...]
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import ts from "typescript";
import {createHash} from "node:crypto";

const root=path.resolve(import.meta.dirname,".."),scratch=path.join(root,"src-tauri/target/quad-eval-runtime");
fs.mkdirSync(scratch,{recursive:true});
fs.writeFileSync(path.join(scratch,"package.json"),'{"type":"commonjs"}');
for(const name of ["quadMatch","quadMask","fireColourMask"]){
  const source=fs.readFileSync(path.join(root,`src/lib/${name}.ts`),"utf8");
  fs.writeFileSync(path.join(scratch,`${name}.js`),ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2020}}).outputText);
}
const require=createRequire(import.meta.url),{matchFireBox}=require(path.join(scratch,"quadMatch.js"));
const sheets=[0,1].map(i=>new Uint8ClampedArray(fs.readFileSync(path.join(root,`../fire-detection/data/assets/fire_${i}.rgba`))));
const same=(a,b)=>a&&a.sheet===b[0]&&a.row===b[1]&&a.mirrored===b[2];
function summary(rows){
  const accepted=rows.filter(r=>r.status==="clear");
  return {parts:rows.length,top1:rows.filter(r=>r.correct).length,top3:rows.filter(r=>r.top3).length,
    accepted:accepted.length,accepted_correct:accepted.filter(r=>r.correct).length,
    accepted_wrong:accepted.filter(r=>!r.correct).length,abstained:rows.length-accepted.length};
}
for(const argument of process.argv.slice(2))for(const stretch of [false,true]){
  const dir=path.resolve(argument),manifest=JSON.parse(fs.readFileSync(path.join(dir,"manifest.json"))),rows=[];
  for(const item of manifest.samples){
    const bytes=fs.readFileSync(path.join(dir,item.file)),data=new Uint8ClampedArray(item.width*item.height*4);
    if(bytes.length!==item.width*item.height*3)throw Error(`Bad crop ${item.file}`);
    for(let i=0;i<bytes.length/3;i++){data.set(bytes.subarray(i*3,i*3+3),i*4);data[i*4+3]=255;}
    const [x0,y0,x1,y1]=item.box;
    const result=matchFireBox({width:item.width,height:item.height,data},sheets,[[x0,y0],[x1,y0],[x1,y1],[x0,y1]],{stretch});
    const top=result.candidates[0];
    rows.push({...item,status:result.status,margin:result.margin,score:top?.score??0,support:top?.support??0,
      prediction:top?[top.sheet,top.row,top.mirrored]:null,correct:same(top,item.expected),
      top3:result.candidates.slice(0,3).some(c=>same(c,item.expected))});
    if(rows.length%200===0)console.error(`${path.basename(dir)}: ${rows.length}/${manifest.samples.length}`);
  }
  const groups={};
  for(const source of ["block","entity","overlay"])groups[source]=summary(rows.filter(r=>r.source===source));
  for(const [name,test] of [["visibility_lt_0.25",r=>r.visibility<0.25],["visibility_0.25_to_0.75",r=>r.visibility>=0.25&&r.visibility<0.75],["visibility_ge_0.75",r=>r.visibility>=0.75],["min_side_lt_16",r=>r.min_pixels<16],["min_side_ge_16",r=>r.min_pixels>=16]])groups[name]=summary(rows.filter(test));
  const matcher_sha256=createHash("sha256").update(fs.readFileSync(path.join(root,"src/lib/quadMatch.ts"))).digest("hex");
  const report={matcher_sha256,stretch,method:"colour-mask-positive-votes; candidate-specific opaque hull; fixed app thresholds; no truth row used for geometry",manifest:{...manifest,samples:undefined},summary:summary(rows),groups,rows};
  fs.writeFileSync(path.join(dir,stretch?"results-stretch.json":"results-rigid.json"),JSON.stringify(report,null,2));
  console.log(JSON.stringify({dataset:path.basename(dir),stretch,summary:report.summary,groups}));
}
