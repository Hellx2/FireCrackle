import { useEffect, useRef, useState, type PointerEvent } from "react";
import { useWheel } from "../hooks/useWheel";
import type { Backend, View } from "../lib/api";
import { IDENTITY, clampView, panView, wheelFactor, zoomAt, type Box, type ViewXform } from "../lib/zoom";
import { displayRow, textureRow, isFire, type Label } from "../lib/grid";
import { formatTime } from "../lib/timeline";
import { HudOverlay } from "./HudOverlay";
import { hudRowFromLabel, stepHudRow } from "../lib/hudOverlay";
import { FireQuadMask } from "./FireQuadMask";
import { DEFAULT_QUAD, type ManualQuad } from "../lib/quadMask";

export function describeLabel(label: Label): string {
  if (label === "unset") return "unlabelled";
  if (label === "no_fire") return "no fire";
  if (label === "skip") return "skipped";
  const { sheet, row, mirrored } = label.fire;
  return `sheet ${sheet} · row ${displayRow(sheet, row)} · ${mirrored ? "mirrored" : "normal"}`;
}

function labelClass(label: Label): string {
  if (isFire(label)) return "fire";
  return label;
}

interface Props {
  backend: Backend;
  view: View;
  onError: (message: string) => void;
}

/** The current frame, with its number, time and label over it. */
export function VideoPane({ backend, view, onError }: Props) {
  const [url, setUrl] = useState<string | null>(null);
  const [shown, setShown] = useState<number | null>(null);
  const token = useRef(0);
  const urlRef = useRef<string | null>(null);
  const [hudEnabled, setHudEnabled] = useState(false);
  const [hudRow, setHudRow] = useState(0);
  const [hudFov, setHudFov] = useState(70);
  const [manualEnabled, setManualEnabled] = useState(false);
  const [manualQuad, setManualQuad] = useState<ManualQuad>(DEFAULT_QUAD);
  const [manualSheet, setManualSheet] = useState<0 | 1>(1);
  const [manualRow, setManualRow] = useState(0);
  const [manualMirrored, setManualMirrored] = useState(false);
  const [manualOpacity, setManualOpacity] = useState(0.6);
  const [imageSize, setImageSize] = useState({ width: 0, height: 0 });
  const labelRow = hudRowFromLabel(view.label);

  useEffect(() => {
    const mine = ++token.current;
    backend
      .frame(view.current)
      .then((bytes) => {
        if (mine !== token.current) return; // a newer frame was asked for meanwhile
        const next = URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" }));
        const old = urlRef.current;
        urlRef.current = next;
        setUrl(next);
        setShown(view.current);
        if (old) URL.revokeObjectURL(old);
      })
      .catch((e) => mine === token.current && onError(`Could not decode frame ${view.current}: ${String(e)}`));
  }, [backend, view.current]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(
    () => () => {
      if (urlRef.current) URL.revokeObjectURL(urlRef.current);
    },
    [],
  );

  // zoom (wheel, around the cursor) and pan (left-drag) of the picture
  const frameRef = useRef<HTMLDivElement | null>(null);
  const [xf, setXf] = useState<ViewXform>(IDENTITY);
  const dragRef = useRef<{ x: number; y: number } | null>(null);
  const boxOf = (): Box => {
    const r = frameRef.current?.getBoundingClientRect();
    return { w: r?.width ?? 0, h: r?.height ?? 0 };
  };

  useWheel(frameRef, (e) => {
    const r = frameRef.current?.getBoundingClientRect();
    if (!r) return false;
    setXf((v) => zoomAt(v, e.clientX - r.left, e.clientY - r.top, wheelFactor(e.deltaY), boxOf()));
    return true;
  });

  // the pane can be resized: keep the view inside it
  useEffect(() => {
    const el = frameRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setXf((v) => clampView(v, boxOf())));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  function panDown(e: PointerEvent<HTMLDivElement>) {
    if (e.button !== 0 || xf.scale <= 1) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    dragRef.current = { x: e.clientX, y: e.clientY };
  }
  function panMove(e: PointerEvent<HTMLDivElement>) {
    const d = dragRef.current;
    if (!d) return;
    const dx = e.clientX - d.x;
    const dy = e.clientY - d.y;
    dragRef.current = { x: e.clientX, y: e.clientY };
    setXf((v) => panView(v, dx, dy, boxOf()));
  }
  function panUp() {
    dragRef.current = null;
  }

  const s = view.suggestion;
  return (
    <div className="video-pane">
      <div
        className={`video-frame${xf.scale > 1 ? " zoomed" : ""}`}
        ref={frameRef}
        onPointerDown={panDown}
        onPointerMove={panMove}
        onPointerUp={panUp}
        onPointerCancel={panUp}
        onDoubleClick={() => setXf(IDENTITY)}
        title="scroll to zoom · drag to move · double-click to reset"
      >
        <div
          className="video-zoom"
          style={{ transform: `translate(${xf.x}px, ${xf.y}px) scale(${xf.scale})` }}
        >
          {url ? <img src={url} alt={`frame ${shown ?? ""}`} draggable={false} onLoad={(e) => setImageSize({ width: e.currentTarget.naturalWidth, height: e.currentTarget.naturalHeight })} /> : <div className="placeholder">decoding…</div>}
          {hudEnabled && url && imageSize.width > 0 && <HudOverlay {...imageSize} row={hudRow} fov={hudFov} opacity={1} />}
          {manualEnabled && url && imageSize.width > 0 && <FireQuadMask {...imageSize} sheet={manualSheet} row={textureRow(manualSheet, manualRow)} mirrored={manualMirrored}
            opacity={manualOpacity} quad={manualQuad} onQuad={setManualQuad} />}
        </div>
        {xf.scale > 1 && <div className="zoom-tag">{xf.scale.toFixed(1)}× · double-click to reset</div>}
        {shown !== view.current && url && <div className="loading-tag">loading frame {view.current}…</div>}
        <div className={`label-chip ${labelClass(view.label)}`}>{describeLabel(view.label)}</div>
        {view.typed_pending !== null && <div className="typed-chip">row: {view.typed_pending}_</div>}
        {s && (
          <div className="suggest-chip" title="Enter accepts the suggestion">
            Enter ▸ sheet {s.sheet} · row {displayRow(s.sheet, s.row)}
            {s.alt_row !== null && <span className="alt"> (or {displayRow(s.sheet, s.alt_row)})</span>}
            {s.mirrored ? " · mirrored" : ""}
          </div>
        )}
      </div>
      <div className="hud-controls" onKeyDown={(e) => e.stopPropagation()}>
        <label><input type="checkbox" checked={hudEnabled} onChange={(e) => { setHudEnabled(e.target.checked); if (e.target.checked) setManualEnabled(false); }} /> First-person fire</label>
        {hudEnabled && <>
          <button aria-label="Previous HUD row" onClick={() => setHudRow((r) => stepHudRow(r, -1))}>◀</button>
          <label>Row <input aria-label="HUD texture row" type="number" min={0} max={31} value={hudRow} onChange={(e) => { const n = e.target.valueAsNumber; if (Number.isInteger(n) && n >= 0 && n < 32) setHudRow(n); }} /></label>
          <button aria-label="Next HUD row" onClick={() => setHudRow((r) => stepHudRow(r, 1))}>▶</button>
          <button disabled={labelRow === null || shown !== view.current} onClick={() => labelRow !== null && setHudRow(labelRow)} title="Copies this frame's animation row into the preview">Use frame label</button>
          <label>FOV <select aria-label="HUD view" value={hudFov} onChange={(e) => setHudFov(Number(e.target.value))}><option value={70}>70° · normal</option><option value={60}>60° · submerged</option></select></label>
          <span className="hud-hint">Fixed first-person preview</span>
        </>}
      </div>
      <div className="hud-controls" onKeyDown={(e) => e.stopPropagation()}>
        <label><input type="checkbox" checked={manualEnabled} onChange={(e) => { setManualEnabled(e.target.checked); if (e.target.checked) setHudEnabled(false); }} /> Manual fire quad mask</label>
        {manualEnabled && <>
          <label>Sheet <select aria-label="Manual mask sheet" value={manualSheet} onChange={(e) => setManualSheet(Number(e.target.value) as 0 | 1)}><option value={0}>0</option><option value={1}>1</option></select></label>
          <button aria-label="Previous manual mask row" onClick={() => setManualRow((r) => stepHudRow(r, -1))}>◀</button>
          <label>Row <input aria-label="Manual mask row" type="number" min={0} max={31} value={manualRow} onChange={(e) => { const n = e.target.valueAsNumber; if (Number.isInteger(n) && n >= 0 && n < 32) setManualRow(n); }} /></label>
          <button aria-label="Next manual mask row" onClick={() => setManualRow((r) => stepHudRow(r, 1))}>▶</button>
          <label><input type="checkbox" checked={manualMirrored} onChange={(e) => setManualMirrored(e.target.checked)} /> Mirrored</label>
          <label>Opacity <input aria-label="Manual mask opacity" type="range" min={0} max={1} step={0.05} value={manualOpacity} onChange={(e) => setManualOpacity(e.target.valueAsNumber)} /></label>
          <button onClick={() => setManualQuad(DEFAULT_QUAD)}>Reset corners</button>
          <button disabled={!isFire(view.label)} onClick={() => { if (isFire(view.label)) { setManualSheet(view.label.fire.sheet as 0 | 1); setManualRow(displayRow(view.label.fire.sheet, view.label.fire.row)); setManualMirrored(view.label.fire.mirrored); } }}>Use frame label</button>
          <span className="hud-hint">drag the four blue corners · reference only, labels unchanged</span>
        </>}
      </div>
      <div className="video-meta">
        <span>
          frame <b>{view.current}</b> / {view.frame_count - 1}
        </span>
        <span>{formatTime(view.pts, 0.01)}</span>
        <span>{view.fps.toFixed(2)} fps</span>
        <span>
          labelled <b>{view.labelled_count}</b>
        </span>
        <span className={view.flag_count > 0 ? "warn" : ""}>flags {view.flag_count}</span>
      </div>
    </div>
  );
}
