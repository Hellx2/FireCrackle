import { useEffect, useRef, useState, type PointerEvent } from "react";
import { useWheel } from "../hooks/useWheel";
import type { Backend, View } from "../lib/api";
import { IDENTITY, clampView, panView, wheelFactor, zoomAt, type Box, type ViewXform } from "../lib/zoom";
import { isFire, type Label } from "../lib/grid";
import { formatTime } from "../lib/timeline";

export function describeLabel(label: Label): string {
  if (label === "unset") return "unlabelled";
  if (label === "no_fire") return "no fire";
  if (label === "skip") return "skipped";
  const { sheet, row, mirrored } = label.fire;
  return `sheet ${sheet} · row ${row} · ${mirrored ? "mirrored" : "normal"}`;
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
          {url ? <img src={url} alt={`frame ${shown ?? ""}`} draggable={false} /> : <div className="placeholder">decoding…</div>}
        </div>
        {xf.scale > 1 && <div className="zoom-tag">{xf.scale.toFixed(1)}× · double-click to reset</div>}
        {shown !== view.current && url && <div className="loading-tag">loading frame {view.current}…</div>}
        <div className={`label-chip ${labelClass(view.label)}`}>{describeLabel(view.label)}</div>
        {view.typed_pending !== null && <div className="typed-chip">row: {view.typed_pending}_</div>}
        {s && (
          <div className="suggest-chip" title="Enter accepts the suggestion">
            Enter ▸ sheet {s.sheet} · row {s.row}
            {s.alt_row !== null && <span className="alt"> (or {s.alt_row})</span>}
            {s.mirrored ? " · mirrored" : ""}
          </div>
        )}
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
