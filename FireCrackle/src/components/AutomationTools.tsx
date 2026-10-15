import { useEffect, useState } from "react";
import type { Backend, PovStatus } from "../lib/api";
import { messageOf } from "../hooks/useWorkspace";
import "./AutomationTools.css";

export function AutomationTools({backend,videoPath,onApply,onClose}:{backend:Backend;videoPath?:string;onApply:(id:number)=>Promise<void>;onClose:()=>void}) {
  const [status,setStatus]=useState<PovStatus|null>(null);
  const [error,setError]=useState<string|null>(null);
  const [pending,setPending]=useState(false);
  useEffect(()=>{
    let live=true;
    const refresh=async()=>{try{const next=await backend.povStatus?.();if(live)setStatus(next && next.video_path===videoPath?next:null);}catch(e){if(live)setError(messageOf(e));}};
    void refresh();const timer=setInterval(()=>void refresh(),700);
    return()=>{live=false;clearInterval(timer);};
  },[backend,videoPath]);
  async function perform(action:()=>Promise<unknown>) {
    setPending(true);setError(null);
    try{await action();const next=await backend.povStatus?.();setStatus(next && next.video_path===videoPath?next:null);}
    catch(e){setError(messageOf(e));}finally{setPending(false);}
  }
  const running=status?.state==="running";
  const supported=!!backend.povStart;
  return <div className="modal-backdrop" onMouseDown={onClose}>
    <div className="modal automation-tools" role="dialog" aria-modal="true" aria-label="Optional automation tools" onMouseDown={e=>e.stopPropagation()}>
      <div className="modal-head"><h2>Optional automation</h2><button onClick={onClose}>Close</button></div>
      <div className="modal-body">
        <h3>First-person fire labeller</h3>
        <p>Finds the fire animation covering the first-person view. Run it whenever you want, after opening a video.</p>
        <p>No false positives were observed in the two tested clips, and every accepted row was correct. You can apply accepted matches in one batch. Use the Phase chart to spot unusual timing: if something looks odd near an automatically labelled frame, check that automatic label first.</p>
        <p className="dim">For the full, uncropped vanilla Minecraft 1.16.5 view. Uncertain frames stay untouched. Existing labels are preserved. Results are marked “First-person tool”, not “Human checked”.</p>
        {!supported&&<p>This tool is available in the desktop app.</p>}
        {status&&<div className="automation-progress" role="status">
          <progress aria-label="First-person scan progress" max={Math.max(1,status.total)} value={status.processed}/>
          <span>{status.state==="running"?"Scanning":status.state==="done"?"Scan complete":status.state==="cancelled"?"Cancelled":"Scan failed"} · {status.processed.toLocaleString()} / {status.total.toLocaleString()} frames</span>
          <span>{status.accepted.toLocaleString()} accepted matches{status.applied>0?` · ${status.applied.toLocaleString()} added`:""}</span>
        </div>}
        {(error||status?.error)&&<p role="alert">{error??status?.error}</p>}
        <div className="automation-actions">
          <button disabled={!supported||running||pending} onClick={()=>void perform(async()=>{setStatus(await backend.povStart!());})}>{status?"Scan again":"Scan video"}</button>
          {running&&<button disabled={pending} onClick={()=>void perform(()=>backend.povCancel!(status!.id))}>Cancel scan</button>}
          {status?.state==="done"&&<button disabled={pending||status.accepted===0||status.applied>0} onClick={()=>void perform(()=>onApply(status.id))}>Apply accepted matches</button>}
        </div>
        <p className="dim">Applying fills empty frames only and can be undone in one step. Closing this panel lets the scan continue.</p>
        {status?.applied ? <p>Open the Phase chart to spot unusual timing. If it looks odd near an automatically labelled frame, check that automatic label first. Use the source filter to isolate tool results. Independently choosing and confirming the same sprite adds “Human checked” while keeping the tool source.</p> : null}
      </div>
    </div>
  </div>;
}
