import { useEffect, useRef, useState } from "react";
import type { Backend, View } from "../lib/api";
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

  const s = view.suggestion;
  return (
    <div className="video-pane">
      <div className="video-frame">
        {url ? <img src={url} alt={`frame ${shown ?? ""}`} draggable={false} /> : <div className="placeholder">decoding…</div>}
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
