import { useEffect, useRef, useState, type PointerEvent } from "react";
import { moveRuler, rulerTicks, type RulerPoints } from "../lib/ruler";
import { screenToQuadPoint, type QuadPoint } from "../lib/quadMask";
import "./VideoRuler.css";

interface Props { width: number; height: number; zoom: number; points: RulerPoints; onChange: (points: RulerPoints) => void }
export function VideoRuler({ width, height, zoom, points, onChange }: Props) {
  const svgRef = useRef<SVGSVGElement>(null);
  const drag = useRef<{ handle: 0 | 1 | "move"; start: QuadPoint; points: RulerPoints } | null>(null);
  const [scale, setScale] = useState(1);
  useEffect(() => {
    const svg = svgRef.current; if (!svg) return;
    const measure = () => { const m = svg.getScreenCTM(); const s = m ? Math.hypot(m.a, m.b) : 0; if (s > 0 && Number.isFinite(s)) setScale(s); };
    measure(); const observer = new ResizeObserver(measure); observer.observe(svg);
    return () => observer.disconnect();
  }, [width, height, zoom]);
  const geometry = rulerTicks(points, width, height, scale);
  function cursor(e: PointerEvent<SVGElement>) {
    const matrix = svgRef.current?.getScreenCTM();
    return matrix ? screenToQuadPoint(matrix, e.clientX, e.clientY, width, height) : null;
  }
  function begin(e: PointerEvent<SVGElement>, handle: 0 | 1 | "move") {
    if (e.button !== 0) return;
    const start = cursor(e); if (!start) return;
    e.preventDefault(); e.stopPropagation();
    drag.current = { handle, start, points };
    e.currentTarget.setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent<SVGSVGElement>) {
    if (!drag.current) return;
    e.preventDefault(); e.stopPropagation();
    const point = cursor(e); if (!point) return;
    const { handle, start, points: initial } = drag.current;
    onChange(moveRuler(initial, handle, [point[0] - start[0], point[1] - start[1]]));
  }
  function stop(e: PointerEvent<SVGSVGElement>) { if (drag.current) e.stopPropagation(); drag.current = null; }
  if (!geometry) return null;
  return <svg ref={svgRef} className="video-ruler" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="xMidYMid meet" aria-label="Calibrated ruler: drag 0 and 1 to define one unit" onPointerMove={move} onPointerUp={stop} onPointerCancel={stop} onLostPointerCapture={stop}>
    <g transform={`translate(${geometry.x} ${geometry.y}) rotate(${geometry.angle})`}>
      <line className="ruler-shadow" x1={geometry.min} x2={geometry.max} strokeWidth={4 / scale} />
      <line className="ruler-axis" x1={geometry.min} x2={geometry.max} strokeWidth={1.5 / scale} />
      {geometry.ticks.map(tick => <g key={tick.index}>
        <line className="ruler-tick" x1={tick.x} x2={tick.x} y1={-5 / scale} y2={(tick.label ? 9 : 5) / scale} strokeWidth={1 / scale} />
        {tick.label && tick.index !== 0 && tick.index !== 1 && <text className="ruler-number" x={tick.x} y={23 / scale} fontSize={11 / scale} strokeWidth={3 / scale}>{tick.index}</text>}
      </g>)}
      <line className="ruler-hit" x1={geometry.min} x2={geometry.max} strokeWidth={14 / scale} onPointerDown={e => begin(e, "move")}><title>Move the ruler without changing its unit</title></line>
    </g>
    {points.map(([x, y], i) => <g key={i} className={`ruler-calibration point-${i}`} transform={`translate(${x * width} ${y * height})`} onPointerDown={e => begin(e, i as 0 | 1)}>
      <circle r={7 / scale} strokeWidth={2 / scale} />
      <text className="ruler-number" y={-14 / scale} fontSize={12 / scale} strokeWidth={3 / scale}>{i}</text>
      <title>{i === 0 ? "0: ruler origin" : "1: one unit from the origin; drag to set scale and direction"}</title>
    </g>)}
  </svg>;
}
