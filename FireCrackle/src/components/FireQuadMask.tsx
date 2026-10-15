import { useEffect, useId, useMemo, useRef, useState, type PointerEvent } from "react";
import fire0 from "../assets/fire_0.png";
import fire1 from "../assets/fire_1.png";
import { manualTexels, moveQuadCorner, screenToQuadPoint, translateQuad, type ManualQuad, type QuadPoint } from "../lib/quadMask";
import "./FireQuadMask.css";
import { FireTexels } from "./FireTexels";

interface Props {
  width: number;
  height: number;
  sheet: 0 | 1;
  row: number;
  mirrored: boolean;
  opacity: number;
  pixelGrid?: boolean;
  zoom?: number;
  quad: ManualQuad;
  onQuad: (quad: ManualQuad) => void;
}

/** Independent manual reference: dragging it never edits a frame label. */
export function FireQuadMask({ width, height, sheet, row, mirrored, opacity, quad, onQuad, pixelGrid = false, zoom = 1 }: Props) {
  const clipId = useId();
  const svgRef = useRef<SVGSVGElement>(null);
  const dragging = useRef<{ corner: number; last: QuadPoint } | null>(null);
  const [selectedCorner, setSelectedCorner] = useState<number | null>(null);
  const latestQuad = useRef(quad);
  latestQuad.current = quad;
  const [rgba, setRgba] = useState<Uint8ClampedArray | null>(null);
  const [error, setError] = useState(false);
  const [screenScale, setScreenScale] = useState(1);
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

  function cursorPoint(x: number, y: number): QuadPoint | null {
    const matrix = svgRef.current?.getScreenCTM();
    return matrix ? screenToQuadPoint(matrix, x, y, width, height) : null;
  }
  function begin(e: PointerEvent<SVGElement>, corner: number, select: boolean) {
    if (e.button !== 0) return;
    const point = cursorPoint(e.clientX, e.clientY);
    if (!point) return;
    e.stopPropagation();
    e.preventDefault();
    if (select) setSelectedCorner(corner);
    dragging.current = { corner, last: point };
    e.currentTarget.setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent<SVGSVGElement>) {
    const drag = dragging.current;
    if (!drag) return;
    e.stopPropagation();
    const point = cursorPoint(e.clientX, e.clientY);
    if (!point) return;
    const next = e.ctrlKey
      ? translateQuad(latestQuad.current, [point[0] - drag.last[0], point[1] - drag.last[1]])
      : moveQuadCorner(latestQuad.current, drag.corner, point);
    drag.last = point;
    latestQuad.current = next;
    onQuad(next);
  }
  function stop(e: PointerEvent<SVGSVGElement>) {
    if (dragging.current !== null) e.stopPropagation();
    dragging.current = null;
  }

  // Alt+click places the selected corner once, including outside the image.
  useEffect(() => {
    const snap = (x: number, y: number) => {
      if (selectedCorner === null || dragging.current) return;
      const matrix = svgRef.current?.getScreenCTM();
      const point = matrix ? screenToQuadPoint(matrix, x, y, width, height) : null;
      if (point) {
        const next = moveQuadCorner(latestQuad.current, selectedCorner, point);
        latestQuad.current = next;
        onQuad(next);
      }
    };
    const click = (e: MouseEvent) => {
      if (!e.altKey || e.button !== 0 || selectedCorner === null || dragging.current) return;
      e.preventDefault();
      e.stopPropagation();
      snap(e.clientX, e.clientY);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") setSelectedCorner(null);
    };
    window.addEventListener("click", click, true);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("click", click, true);
      window.removeEventListener("keydown", key);
    };
  }, [width, height, selectedCorner, onQuad]);

  // Convert a screen-pixel handle size into SVG units, including image fit and zoom.
  useEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;
    const measure = () => {
      const matrix = svg.getScreenCTM();
      if (!matrix) return;
      const scale = Math.hypot(matrix.a, matrix.b);
      if (Number.isFinite(scale) && scale > 0) setScreenScale(scale);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(svg);
    return () => observer.disconnect();
  }, [width, height, zoom]);
  return <>
    {error && <div className="hud-error" role="alert">Could not load the manual fire sprite.</div>}
    <svg ref={svgRef} className="fire-quad-mask" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="xMidYMid meet"
      aria-label="Manual fire matching mask: drag the four corner handles" onPointerMove={move} onPointerUp={stop} onPointerCancel={stop} onLostPointerCapture={stop}>
      <defs><clipPath id={clipId}><rect width={width} height={height} /></clipPath></defs>
      <g opacity={opacity} clipPath={`url(#${clipId})`}>
        <FireTexels texels={texels} pixelGrid={pixelGrid} scaleX={width} scaleY={height} />
      </g>
      <polygon className="fire-quad-guide" points={points(quad)} strokeWidth={2 / screenScale} onPointerDown={(e) => { if (e.ctrlKey) begin(e, selectedCorner ?? 0, false); }} />
      {quad.map(([x, y], i) => <circle key={i} className={`fire-quad-handle${selectedCorner === i ? " selected" : ""}`} cx={x * width} cy={y * height}
        r={8 / screenScale} fill="#61e9ff" stroke="#10202a" strokeWidth={2 / screenScale}
        onPointerDown={(e) => begin(e, i, true)}>
        <title>{`Corner ${i + 1}: drag to move; Ctrl+drag moves all; select then Alt+click to place`}</title>
      </circle>)}
    </svg>
  </>;
}
