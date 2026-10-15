"""Extract native-pixel crops from joined oracle bbox labels; no synthetic images.

python scripts/prepare-quad-eval.py VIDEO JOINED OUT --ffmpeg PATH --stride 300
Frame n is the decoded video index. Truth is stored only for later scoring.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("video", type=Path)
    ap.add_argument("joined", type=Path)
    ap.add_argument("out", type=Path)
    ap.add_argument("--ffmpeg", required=True, type=Path)
    ap.add_argument("--stride", type=int, default=300)
    args = ap.parse_args()
    if args.stride <= 0:
        ap.error("stride must be positive")
    probe = args.ffmpeg.with_name("ffprobe.exe" if args.ffmpeg.suffix == ".exe" else "ffprobe")
    meta = json.loads(subprocess.check_output([str(probe), "-v", "error", "-select_streams", "v:0",
        "-show_entries", "stream=width,height", "-of", "json", str(args.video)]))["streams"][0]
    width, height = meta["width"], meta["height"]
    labels = {}
    with args.joined.open(encoding="utf-8") as file:
        for line in file:
            row = json.loads(line)
            n = row["n"]
            if n % args.stride == 0:
                if n in labels:
                    raise ValueError(f"Duplicate frame {n}")
                labels[n] = row
    args.out.mkdir(parents=True, exist_ok=True)
    command = [str(args.ffmpeg), "-v", "error", "-i", str(args.video), "-vf",
        f"select=not(mod(n\\,{args.stride}))", "-fps_mode", "passthrough", "-f", "rawvideo",
        "-pix_fmt", "rgb24", "pipe:1"]
    process = subprocess.Popen(command, stdout=subprocess.PIPE)
    samples, frames, skipped_sync, invalid = [], 0, 0, 0
    try:
        while True:
            raw = process.stdout.read(width * height * 3)
            if not raw:
                break
            if len(raw) != width * height * 3:
                raise ValueError("Truncated decoded frame")
            n = frames * args.stride
            frames += 1
            joined = labels.get(n)
            if not joined or not joined.get("tick_ok"):
                skipped_sync += 1
                continue
            label = joined["label"]
            for fi, fire in enumerate(label.get("fires", [])):
                for pi, part in enumerate(fire.get("parts", [])):
                    x0, y0, x1, y1 = part["box"]
                    x0, x1 = x0 * width / label["w"], x1 * width / label["w"]
                    y0, y1 = y0 * height / label["h"], y1 * height / label["h"]
                    if not all(math.isfinite(v) for v in [x0,y0,x1,y1]) or x1 <= x0 or y1 <= y0:
                        invalid += 1
                        continue
                    pad_x, pad_y = (x1-x0)*0.3+2, (y1-y0)*0.3+2
                    left, top = max(0, math.floor(x0-pad_x)), max(0, math.floor(y0-pad_y))
                    right, bottom = min(width, math.ceil(x1+pad_x)), min(height, math.ceil(y1+pad_y))
                    if right <= left or bottom <= top:
                        invalid += 1
                        continue
                    cw, ch = right-left, bottom-top
                    # Lossless native pixels; cropping does not resize or recompress.
                    crop = b"".join(raw[(y*width+left)*3:(y*width+right)*3] for y in range(top,bottom))
                    name = f"{n}-{fi}-{pi}.rgb"
                    (args.out/name).write_bytes(crop)
                    samples.append(dict(file=name, frame=n, width=cw, height=ch,
                        box=[(x0-left)/cw,(y0-top)/ch,(x1-left)/cw,(y1-top)/ch],
                        expected=[part["sheet"],part["row"],part["mirror"]],
                        source=fire["src"], visibility=part["vis"], veil=part.get("veil",0),
                        min_pixels=min(x1-x0,y1-y0)))
            if frames % 20 == 0:
                print(f"Decoded {frames} sampled frames; {len(samples)} labelled parts", flush=True)
    finally:
        process.stdout.close()
        if process.poll() is None:
            process.terminate()
        code = process.wait()
    if code != 0:
        raise RuntimeError(f"ffmpeg exit {code}")
    manifest = dict(video=str(args.video.resolve()), joined=str(args.joined.resolve()),
        joined_sha256=hashlib.sha256(args.joined.read_bytes()).hexdigest(),
        native_size=[width,height], stride=args.stride, frames=frames,
        skipped_sync=skipped_sync, invalid_boxes=invalid, samples=samples)
    (args.out/"manifest.json").write_text(json.dumps(manifest),encoding="utf-8")
    print(f"Saved {len(samples)} real labelled crops to {args.out}")


if __name__ == "__main__":
    main()
