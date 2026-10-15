import { useEffect, useRef, useState } from "react";
import type { Backend, View } from "../lib/api";
import {
  flagSpans,
  hitFlag,
  nearestPoint,
  phaseToY,
  relativePhase,
  rowToY,
  timeToX,
  xToTime,
  type PhaseChart,
} from "../lib/chart";
import { formatTime } from "../lib/timeline";
import { messageOf } from "../hooks/useWorkspace";
import { FlagsPanel } from "./FlagsPanel";

/** Most points the backend sends: a bit more than the pixels a wide window has. */
const MAX_POINTS = 4000;
const COLORS = { hard: "#ff4d4f", soft: "#f5c542", dot: "#4a90d9", band: "rgba(76,195,138,0.16)", grid: "#2a3142", text: "#8b96ab" };

interface Props {
  backend: Backend;
  view: View;
  /** Changes whenever labels changed. */
  version: number;
  /** Jump to a frame in the Label tab. */
  onJump: (frame: number) => void;
}

/** The Phase tab: phase against time, the sprite row against time, and every flag, clickable. */
export function PhaseTab({ backend, view, version, onJump }: Props) {
  const [chart, setChart] = useState<PhaseChart | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [hover, setHover] = useState<string | null>(null);
  const phaseRef = useRef<HTMLCanvasElement>(null);
  const rowRef = useRef<HTMLCanvasElement>(null);
  const fps = view.fps || 1;
  const t1 = Math.max(view.frame_count - 1, 1) / fps;

  useEffect(() => {
    let cancelled = false;
    backend
      .phaseChart(MAX_POINTS)
      .then((c) => !cancelled && (setChart(c), setError(null)))
      .catch((e) => !cancelled && setError(messageOf(e)));
    return () => {
      cancelled = true;
    };
  }, [backend, version]);

  const timeOf = (frame: number) => frame / fps;

  function draw(canvas: HTMLCanvasElement | null, kind: "phase" | "row") {
    if (!canvas || !chart) return;
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    canvas.width = Math.max(1, Math.round(w * dpr));
    canvas.height = Math.max(1, Math.round(h * dpr));
    const g = canvas.getContext("2d");
    if (!g) return;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, w, h);
    g.font = "11px system-ui, sans-serif";
    // flags first, behind the dots
    for (const [i, s] of flagSpans(chart.flags, timeOf, 0, t1, w).entries()) {
      g.fillStyle = chart.flags[i].severity === "hard" ? "rgba(255,77,79,0.28)" : "rgba(245,197,66,0.22)";
      g.fillRect(s.x0, 0, s.x1 - s.x0, h);
    }
    const ref = chart.runs[0]?.center ?? chart.points[0]?.phase ?? 0;
    if (kind === "phase") {
      g.strokeStyle = COLORS.grid;
      g.fillStyle = COLORS.text;
      for (const rel of [-16, -8, 0, 8, 16]) {
        const y = phaseToY(rel, h);
        g.beginPath();
        g.moveTo(0, y);
        g.lineTo(w, y);
        g.stroke();
        g.fillText(`${rel > 0 ? "+" : ""}${rel}`, 4, Math.min(h - 3, Math.max(11, y - 2)));
      }
      // each run's arc as a band: the stretch a steady clock would stay in
      g.fillStyle = COLORS.band;
      for (const r of chart.runs) {
        const x0 = timeToX(timeOf(r.first_frame), 0, t1, w);
        const x1 = Math.max(timeToX(timeOf(r.last_frame), 0, t1, w), x0 + 1);
        const rel = relativePhase(r.center, ref);
        const top = phaseToY(rel + r.width / 2, h);
        const bottom = phaseToY(rel - r.width / 2, h);
        g.fillRect(x0, top, x1 - x0, Math.max(1, bottom - top));
      }
    } else {
      g.strokeStyle = COLORS.grid;
      g.fillStyle = COLORS.text;
      for (const row of [0, 8, 16, 24, 31]) {
        const y = rowToY(row, h);
        g.beginPath();
        g.moveTo(0, y);
        g.lineTo(w, y);
        g.stroke();
        g.fillText(`row ${row}`, 4, Math.min(h - 3, Math.max(11, y - 2)));
      }
    }
    g.fillStyle = COLORS.dot;
    for (const p of chart.points) {
      const x = timeToX(p.pts, 0, t1, w);
      const y = kind === "phase" ? phaseToY(relativePhase(p.phase, ref), h) : rowToY(p.step, h);
      g.fillRect(x - 1, y - 1, 2.5, 2.5);
    }
    // where the Label tab currently is
    const cx = timeToX(timeOf(view.current), 0, t1, w);
    g.strokeStyle = "#f08a24";
    g.beginPath();
    g.moveTo(cx, 0);
    g.lineTo(cx, h);
    g.stroke();
  }

  useEffect(() => {
    const redraw = () => {
      draw(phaseRef.current, "phase");
      draw(rowRef.current, "row");
    };
    redraw();
    const ro = new ResizeObserver(redraw);
    if (phaseRef.current) ro.observe(phaseRef.current);
    return () => ro.disconnect();
  }, [chart, view.current, t1]); // eslint-disable-line react-hooks/exhaustive-deps

  function locate(e: React.PointerEvent<HTMLCanvasElement>) {
    if (!chart) return null;
    const r = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - r.left;
    const spans = flagSpans(chart.flags, timeOf, 0, t1, r.width);
    const flagIdx = hitFlag(spans, x);
    const t = xToTime(x, 0, t1, r.width);
    return { flagIdx, point: nearestPoint(chart.points, t), t };
  }

  function onMove(e: React.PointerEvent<HTMLCanvasElement>) {
    const hit = locate(e);
    if (!hit) return;
    if (hit.flagIdx !== null) {
      const f = chart!.flags[hit.flagIdx];
      setHover(`${f.severity} flag · frame ${f.frame}${f.end_frame > f.frame ? `–${f.end_frame}` : ""} · Δ ${f.phase_delta.toFixed(1)} ticks · click to open`);
    } else if (hit.point) {
      const p = hit.point;
      setHover(`frame ${p.frame} · ${formatTime(p.pts, 0.01)} · row ${p.step} · phase ${p.phase.toFixed(2)}`);
    } else setHover(null);
  }

  function onClick(e: React.PointerEvent<HTMLCanvasElement>) {
    const hit = locate(e);
    if (!hit) return;
    if (hit.flagIdx !== null) onJump(chart!.flags[hit.flagIdx].frame);
    else if (hit.point) onJump(hit.point.frame);
  }

  return (
    <main className="phase-tab">
      {error && <div className="banner error">{error}</div>}
      <section className="chart-area">
        <div className="chart-head">
          <span className="panel-title">Phase against time</span>
          <span className="dim">
            flat = a steady clock (20 ticks/s). Each shaded band is one run; a step is a cut, a reload or a lag drop. Red/amber = flags.
          </span>
        </div>
        {chart && chart.points.length === 0 ? (
          <div className="empty big-empty">No fire frames labelled yet. Label some in the Label tab and the chart appears here.</div>
        ) : (
          <>
            <canvas ref={phaseRef} className="chart phase" onPointerMove={onMove} onPointerLeave={() => setHover(null)} onPointerUp={onClick} />
            <div className="chart-head">
              <span className="panel-title">Sprite row against time</span>
              <span className="dim">the sawtooth you should see: 32 rows, one step per tick (about every 3rd frame at 60 fps).</span>
            </div>
            <canvas ref={rowRef} className="chart row" onPointerMove={onMove} onPointerLeave={() => setHover(null)} onPointerUp={onClick} />
          </>
        )}
        <div className="chart-hover">{hover ?? " "}</div>
      </section>
      <FlagsPanel flags={chart?.flags ?? []} fps={fps} current={view.current} onSeek={onJump} />
    </main>
  );
}
