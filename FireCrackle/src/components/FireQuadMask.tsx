import { useEffect, useMemo, useRef, useState, type PointerEvent } from "react";
import fire0 from "../assets/fire_0.png";
import fire1 from "../assets/fire_1.png";
import { manualTexels, moveQuadCorner, type ManualQuad } from "../lib/quadMask";
import "./FireQuadMask.css";

interface Props {
  width: number;
  height: number;
  sheet: 0 | 1;
  row: number;
  mirrored: boolean;
  opacity: number;
  quad: ManualQuad;
  onQuad: (quad: ManualQuad) => void;
}

/** Independent manual reference: dragging it never edits a frame label. */
export function FireQuadMask({ width, height, sheet, row, mirrored, opacity, quad, onQuad }: Props) {
  const svgRef = useRef<SVGSVGElement>(null);
  const dragging = useRef<number | null>(null);
  const [rgba, setRgba] = useState<Uint8ClampedArray | null>(null);
  const [error, setError] = useState(false);
  useEffect(() => {
    let cancelled = false;
    setRgba(null);
    setError(false);
    const image = new Image();
    image.onload = () => {
      if (cancelled) return;
      try {
        if (image.naturalWidth !== 16 || image.naturalHeight !== 512) throw new Error("Invalid sprite sheet");
        const canvas = document.createElement("canvas");
        canvas.width = 16;
        canvas.height = 512;
        const ctx = canvas.getContext("2d");
        if (!ctx) throw new Error("Canvas unavailable");
        ctx.drawImage(image, 0, 0);
        setRgba(ctx.getImageData(0, 0, 16, 512).data);
      } catch { setError(true); }
    };
    image.onerror = () => { if (!cancelled) setError(true); };
    image.src = sheet === 0 ? fire0 : fire1;
    return () => { cancelled = true; };
  }, [sheet]);
  const texels = useMemo(() => rgba ? manualTexels(rgba, row, mirrored, quad) : [], [rgba, row, mirrored, quad]);
  const points = (p: Array<[number, number]>) => p.map(([x, y]) => `${x * width},${y * height}`).join(" ");
  function move(e: PointerEvent<SVGSVGElement>) {
    if (dragging.current === null) return;
    e.stopPropagation();
    const matrix = svgRef.current?.getScreenCTM();
    if (!matrix) return;
    const point = new DOMPoint(e.clientX, e.clientY).matrixTransform(matrix.inverse());
    onQuad(moveQuadCorner(quad, dragging.current, [point.x / width, point.y / height]));
  }
  function stop(e: PointerEvent<SVGSVGElement>) {
    if (dragging.current !== null) e.stopPropagation();
    dragging.current = null;
  }
  return <>
    {error && <div className="hud-error" role="alert">Could not load the manual fire sprite.</div>}
    <svg ref={svgRef} className="fire-quad-mask" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="xMidYMid meet"
      aria-label="Manual fire matching mask: drag the four corner handles" onPointerMove={move} onPointerUp={stop} onPointerCancel={stop} onLostPointerCapture={stop}>
      <g opacity={opacity}>{texels.map((p, i) => <polygon key={i} points={points(p.points)} fill={p.colour} fillOpacity={p.alpha} />)}</g>
      <polygon className="fire-quad-guide" points={points(quad)} />
      {quad.map(([x, y], i) => <circle key={i} className="fire-quad-handle" cx={x * width} cy={y * height}
        r={Math.max(width, height) / 120} fill="#61e9ff" stroke="#10202a" strokeWidth={2} vectorEffect="non-scaling-stroke"
        onPointerDown={(e) => { if (e.button !== 0) return; e.stopPropagation(); e.preventDefault(); dragging.current = i; e.currentTarget.setPointerCapture(e.pointerId); }}>
        <title>Drag corner {i + 1}</title>
      </circle>)}
    </svg>
  </>;
}
