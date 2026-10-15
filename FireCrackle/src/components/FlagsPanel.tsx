import type { Flag, FlagKind } from "../lib/api";
import { formatTime } from "../lib/timeline";

const KIND: Record<FlagKind, { name: string; hint: string }> = {
  phase_excursion: { name: "Lag stall", hint: "the phase left its level and came back (the game stalled, then caught up)" },
  phase_shift: { name: "Phase shift", hint: "the phase changed and stayed (a cut, a reload, leaving the world, or a wrong label from here on)" },
  phase_outlier: { name: "Odd frame", hint: "a short stretch off its neighbours (probably a mislabelled frame)" },
};

interface Props {
  flags: Flag[];
  fps: number;
  current: number;
  onSeek: (frame: number) => void;
}

/** The phase flags, each clickable to jump to that frame. */
export function FlagsPanel({ flags, fps, current, onSeek }: Props) {
  return (
    <div className="flags">
      <div className="panel-title">
        Phase flags <span className="count">{flags.length}</span>
      </div>
      {flags.length === 0 && <div className="empty">Nothing suspicious in the labelled frames.</div>}
      <ul>
        {flags.map((f) => {
          const inside = current >= f.frame && current <= f.end_frame;
          return (
            <li key={`${f.frame}-${f.kind}`} className={inside ? "inside" : ""}>
              <button onClick={() => onSeek(f.frame)} title={KIND[f.kind].hint} tabIndex={-1}>
                <span className={`sev ${f.severity}`}>{f.severity}</span>
                <span className="kind">{KIND[f.kind].name}</span>
                <span className="where">
                  frame {f.frame}
                  {f.end_frame > f.frame ? `–${f.end_frame}` : ""} · {formatTime(f.frame / (fps || 1), 0.01)}
                </span>
                <span className="delta">Δ {f.phase_delta > 0 ? "+" : ""}{f.phase_delta.toFixed(1)} ticks</span>
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
