"""Freeze native video frames and REAL quad geometry for temporal evaluation.

No grouped boxes, truth-derived candidate filtering, resizing or recompression.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess


def canonical_quad(item, width, height):
    """Use clockwise screen winding anchored at the texture's top edge.

    UVs establish which physical edge is the top, not a candidate row/sheet.
    No row, sheet or mirror truth is used to choose the vertex order.
    """
    corners, uv = item.get("c"), item.get("uv")
    if corners is None or uv is None or len(corners) != 8 or len(uv) != 8:
        return None, "missing"
    if not all(math.isfinite(v) for v in corners+uv):
        return None, "nonfinite"
    points = list(zip(corners[::2],corners[1::2]))
    area = sum(points[i][0]*points[(i+1)%4][1]-points[(i+1)%4][0]*points[i][1] for i in range(4))/2
    if area >= -30:
        return None, "backface_or_small"
    # Preserve actual geometry; never clamp a corner into a different quad.
    if any(abs(x)>16*width or abs(y)>16*height for x,y in points):
        return None, "extreme_projection"
    # Reverse the screen winding, then anchor that cycle at the texture's
    # top edge. Sorting by screen x would silently flip rotated floor quads.
    cycle=[0,3,2,1]
    starts=[j for j in range(4) if uv[cycle[j]*2+1]<.5 and uv[cycle[(j+1)%4]*2+1]<.5]
    if len(starts)!=1:
        return None, "unsupported_uv"
    j=starts[0];order=cycle[j:]+cycle[:j]
    q=[points[i] for i in order]
    for i in range(4):
        a,b,c=q[i],q[(i+1)%4],q[(i+2)%4]
        if (b[0]-a[0])*(c[1]-b[1])-(b[1]-a[1])*(c[0]-b[0])<=0:
            return None, "degenerate_projection"
    mirror=uv[order[0]*2]>uv[order[1]*2]
    if mirror != bool(item["m"]):
        return None, "mirror_convention"
    return q, None


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument("video",type=Path);ap.add_argument("joined",type=Path);ap.add_argument("out",type=Path)
    ap.add_argument("--ffmpeg",required=True,type=Path);ap.add_argument("--stride",type=int,default=60)
    args=ap.parse_args()
    if args.stride<1:ap.error("stride must be positive")
    probe=args.ffmpeg.with_name("ffprobe.exe" if args.ffmpeg.suffix==".exe" else "ffprobe")
    stream=json.loads(subprocess.check_output([str(probe),"-v","error","-select_streams","v:0","-show_entries","stream=width,height","-of","json",str(args.video)]))["streams"][0]
    w,h=stream["width"],stream["height"]
    labels={}
    for line in args.joined.open(encoding="utf-8"):
        row=json.loads(line)
        if row["n"]%args.stride==0:
            if row["n"] in labels:raise ValueError("duplicate frame")
            labels[row["n"]]=row
    args.out.mkdir(parents=True,exist_ok=True)
    process=subprocess.Popen([str(args.ffmpeg),"-v","error","-i",str(args.video),"-vf",f"select=not(mod(n\\,{args.stride}))","-fps_mode","passthrough","-pix_fmt","rgb24","-f","rawvideo","pipe:1"],stdout=subprocess.PIPE)
    samples=[];skips={};frames=0
    while True:
        raw=process.stdout.read(w*h*3)
        if not raw:break
        if len(raw)!=w*h*3:raise ValueError("truncated frame")
        n=frames*args.stride;frames+=1
        row=labels.get(n)
        if not row or not row.get("tick_ok"):
            skips["unsynced"]=skips.get("unsynced",0)+1;continue
        label=row["label"];cases=[]
        for fi,fire in enumerate(label.get("fires",[])):
            for qi,item in enumerate(fire.get("quads",[])):
                points,error=canonical_quad(item,label["w"],label["h"])
                if error:skips[error]=skips.get(error,0)+1;continue
                quad=[[x/label["w"],y/label["h"]] for x,y in points]
                if max(p[0] for p in quad)<=0 or min(p[0] for p in quad)>=1 or max(p[1] for p in quad)<=0 or min(p[1] for p in quad)>=1:
                    skips["offscreen"]=skips.get("offscreen",0)+1;continue
                split="tune" if n<4136 else "validation" if n>=4376 else "gap"
                cases.append(dict(id=f"{n}-{fi}-{qi}",frame=n,quad=quad,expected=[item["s"],item["row"],bool(item["m"])],source=fire["src"],visibility=item["vis"],veil=item["veil"],split=split))
        if cases:
            (args.out/f"{n}.rgb").write_bytes(raw);samples.extend(cases)
        if frames%20==0:print(f"Prepared {frames} frames; {len(samples)} actual quads",flush=True)
    process.stdout.close()
    if process.wait()!=0:raise RuntimeError("ffmpeg failed")
    manifest=dict(video=str(args.video.resolve()),joined=str(args.joined.resolve()),joined_sha256=hashlib.sha256(args.joined.read_bytes()).hexdigest(),width=w,height=h,stride=args.stride,frames=frames,skips=skips,split=dict(tune_end=4136,validation_start=4376),samples=samples)
    (args.out/"manifest.json").write_text(json.dumps(manifest),encoding="utf-8")
    print(json.dumps(dict(frames=frames,samples=len(samples),skips=skips)))


if __name__=="__main__":main()
