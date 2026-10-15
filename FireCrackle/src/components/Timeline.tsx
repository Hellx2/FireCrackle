import { useEffect, useRef, useState } from "react";
import type { Backend, Bucket, Flag, SettingInfo, View } from "../lib/api";
import { settingValue } from "../lib/api";
import {
  clampViewport,
  ensureVisible,
  fitViewport,
  frameToX,
  pan,
  rulerTicks,
  visibleRange,
  xToFrame,
  zoomAt,
  type Viewport,
} from "../lib/timeline";

const RULER_H = 26;
const LANE_Y = 32;
const LANE_H = 44;
const FLAG_Y = LANE_Y + LANE_H + 4;
const HEIGHT = 108;

const COLORS = {
  bg: "#12151c",
  lane: "#1d2330",
  fire: "#f08a24",
  noFire: "#4a90d9",
  skip: "#7a7f8c",
  hard: "#ff4d4f",
  soft: "#f5c542",
  tick: "#5b6577",
  text: "#9aa5b8",
  playhead: "#ffffff",
};

interface Props {
  backend: Backend;
  view: View;
  flags: Flag[];
  version: number;
  settings: SettingInfo[];
  onSeek: (frame: number) => void;
}

/** Editor-style timeline: time ruler, label lane, flag marks and the playhead. Wheel zooms, drag pans. */
export function Timeline({ backend, view, flags, version, settings, onSeek }: Props) {
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [width, setWidth] = useState(900);
  const total = view.frame_count;
  const [vp, setVp] = useState<Viewport>(() => fitViewport(total, 900));
  const [buckets, setBuckets] = useState<{ first: number; len: number; list: Bucket[] }>({ first: 0, len: 1, list: [] });
  const drag = useRef<{ x0: number; start0: number; moved: boolean } | null>(null);

  const zoomFactor = settingValue(settings, "timeline.zoom_factor", 1.25);
  const maxPx = settingValue(settings, "timeline.max_px_per_frame", 32);
  const margin = settingValue(settings, "timeline.follow_margin_px", 80);
  const spacing = settingValue(settings, "timeline.ruler_spacing_px", 90);
  const dragThreshold = settingValue(settings, "timeline.drag_threshold_px", 4);

  // track the width of the container
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => setWidth(Math.max(200, Math.floor(entries[0].contentRect.width))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // a new video: show all of it; a new width: keep the view valid
  useEffect(() => setVp(fitViewport(total, width)), [total]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => setVp((v) => clampViewport(v, width, total)), [width, total]);

  // follow the playhead (not while the user is dragging the timeline)
  useEffect(() => {
    if (drag.current?.moved) return;
    setVp((v) => ensureVisible(v, view.current, width, total, Math.min(margin, width / 4)));
  }, [view.current, width, total, margin]);

  // lane data for the visible frames, one bucket per pixel column
  useEffect(() => {
    const { first, last } = visibleRange(vp, width, total);
    const n = Math.max(1, Math.floor(width));
    let cancelled = false;
    const timer = setTimeout(() => {
      backend
        .timeline(first, last + 1, n)
        .then((list) => !cancelled && setBuckets({ first, len: last + 1 - first, list }))
        .catch(() => undefined);
    }, 40);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [backend, vp, width, total, version]);

  // draw
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.floor(width * dpr);
    canvas.height = Math.floor(HEIGHT * dpr);
    canvas.style.width = `${width}px`;
    canvas.style.height = `${HEIGHT}px`;
    const g = canvas.getContext("2d");
    if (!g) return;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.fillStyle = COLORS.bg;
    g.fillRect(0, 0, width, HEIGHT);

    // ruler
    g.font = "11px system-ui, sans-serif";
    g.textBaseline = "top";
    for (const t of rulerTicks(vp, width, view.fps, spacing)) {
      g.strokeStyle = COLORS.tick;
      g.beginPath();
      g.moveTo(Math.round(t.x) + 0.5, t.major ? 4 : 12);
      g.lineTo(Math.round(t.x) + 0.5, RULER_H);
      g.stroke();
      if (t.major || spacing >= 40) {
        g.fillStyle = COLORS.text;
        g.fillText(t.label, t.x + 3, 2);
      }
    }

    // label lane
    g.fillStyle = COLORS.lane;
    g.fillRect(frameToX(vp, 0), LANE_Y, Math.max(1, total * vp.pxPerFrame), LANE_H);
    const { first, len, list } = buckets;
    const n = list.length;
    list.forEach((b, i) => {
      if (b.frames === 0) return;
      const x0 = frameToX(vp, first + (i * len) / n);
      const w = Math.max(1, (len / n) * vp.pxPerFrame);
      const labelled = b.fire + b.no_fire + b.skip;
      if (labelled > 0) {
        g.globalAlpha = 0.35 + 0.65 * (labelled / b.frames);
        g.fillStyle = b.fire >= b.no_fire && b.fire >= b.skip ? COLORS.fire : b.no_fire >= b.skip ? COLORS.noFire : COLORS.skip;
        g.fillRect(x0, LANE_Y, w, LANE_H);
        g.globalAlpha = 1;
      }
      if (b.flag) {
        g.fillStyle = b.flag === "hard" ? COLORS.hard : COLORS.soft;
        g.fillRect(x0, FLAG_Y, w, 8);
      }
    });

    // playhead
    const px = frameToX(vp, view.current) + vp.pxPerFrame / 2;
    g.fillStyle = COLORS.playhead;
    g.fillRect(Math.round(px) - 1, 0, 2, HEIGHT);
    g.beginPath();
    g.moveTo(px - 6, 0);
    g.lineTo(px + 6, 0);
    g.lineTo(px, 9);
    g.closePath();
    g.fill();
  }, [vp, width, buckets, flags, view.current, view.fps, total, spacing]);

  // wheel zoom needs a non-passive listener so the page does not scroll
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const x = e.clientX - canvas.getBoundingClientRect().left;
      const factor = e.deltaY < 0 ? zoomFactor : 1 / zoomFactor;
      setVp((v) => zoomAt(v, x, factor, maxPx, width, total));
    };
    canvas.addEventListener("wheel", onWheel, { passive: false });
    return () => canvas.removeEventListener("wheel", onWheel);
  }, [zoomFactor, maxPx, width, total]);

  const localX = (e: React.PointerEvent) => e.clientX - (e.currentTarget as HTMLElement).getBoundingClientRect().left;

  return (
    <div className="timeline" ref={wrapRef}>
      <canvas
        ref={canvasRef}
        onPointerDown={(e) => {
          (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
          drag.current = { x0: localX(e), start0: vp.start, moved: false };
        }}
        onPointerMove={(e) => {
          const d = drag.current;
          if (!d) return;
          const dx = localX(e) - d.x0;
          if (!d.moved && Math.abs(dx) < dragThreshold) return;
          d.moved = true;
          setVp(pan({ start: d.start0, pxPerFrame: vp.pxPerFrame }, dx, width, total));
        }}
        onPointerUp={(e) => {
          const d = drag.current;
          drag.current = null;
          if (d && !d.moved) onSeek(xToFrame(vp, localX(e), total));
        }}
        onDoubleClick={() => setVp(fitViewport(total, width))}
      />
      <div className="timeline-hint">wheel: zoom · drag: pan · click: seek · double-click: show all · {vp.pxPerFrame.toFixed(3)} px/frame</div>
    </div>
  );
}
