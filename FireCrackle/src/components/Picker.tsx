import { useEffect, useRef, useState, type CSSProperties } from "react";
import fire0 from "../assets/fire_0.png";
import fire1 from "../assets/fire_1.png";
import { COLUMNS, ROWS, columnHeader, labelToCell, suggestionCells, type Label } from "../lib/grid";
import type { PreviewSprite } from "../lib/firePreview";
import type { View } from "../lib/api";
import { pickerLayout } from "../lib/pickerLayout";
import "./Picker.css";

interface Props {
  view: View;
  candidate?: Label | null;
  preview?: PreviewSprite | null;
  /** sprite size in px; the 16 px sprites are shown scaled */
  cell: number;
  /** The animation index is shared by both sheets and the backend. */
  onCell: (col: number, row: number, ctrlKey: boolean) => void;
}

/** The 32 sprite rows, divided into equal blocks to fit the panel. */
export function Picker({ view, cell, onCell, candidate = null, preview = null }: Props) {
  const pickerRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  useEffect(() => {
    const picker = pickerRef.current;
    if (!picker) return;
    const observer = new ResizeObserver(([entry]) => {
      setSize({ width: entry.contentRect.width, height: entry.contentRect.height });
    });
    observer.observe(picker);
    return () => observer.disconnect();
  }, []);
  const { blocks, rowsPerBlock, cell: CELL } = pickerLayout(size.width, size.height, cell);
  const current = labelToCell(view.label);
  const chosen = candidate ? labelToCell(candidate) : null;
  const shown = preview ? labelToCell({ fire: preview }) : null;
  const sug = view.suggestion ? suggestionCells(view.suggestion) : { primary: null, alt: null };
  const targetRef = useRef<HTMLDivElement | null>(null);

  const focusRow = chosen?.row ?? shown?.row ?? sug.primary?.row ?? current?.row ?? null;
  useEffect(() => {
    targetRef.current?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [focusRow, blocks, CELL]);

  return (
    <div ref={pickerRef} className="picker picker-adaptive" style={{ "--cell": `${CELL}px`, "--blocks": blocks } as CSSProperties}>
      <div className="picker-blocks">
        {Array.from({ length: blocks }, (_, block) => (
          <div key={block} className="picker-block">
            <div className="picker-head">
              <div className="gutter" />
              <div className="group">Normal</div>
              <div className="group">Mirrored</div>
            </div>
            <div className="picker-head sheets">
              <div className="gutter" />
              {COLUMNS.map((_, c) => (
                <div key={c} className="sheet">
                  {columnHeader(c)?.sheet}
                </div>
              ))}
            </div>
            <div className="picker-rows">
              {Array.from({ length: rowsPerBlock }, (_, offset) => block * rowsPerBlock + offset).map((row) => (
                <div key={row} className="picker-row" ref={row === focusRow ? targetRef : undefined}>
                  <div className="gutter">{row}</div>
                  {COLUMNS.map((col, c) => {
                    const isCurrent = current?.col === c && current.row === row;
                    const isSug = sug.primary?.col === c && sug.primary.row === row;
                    const isAlt = sug.alt?.col === c && sug.alt.row === row;
                    const isChosen = chosen?.col === c && chosen.row === row;
                    const isShown = shown?.col === c && shown.row === row;
                    const states = [isChosen && "Candidate", isShown && "Preview", isSug && `Time guess from frame ${view.suggestion?.from_frame}`, isAlt && "Alternate time guess", isCurrent && "Saved label"].filter(Boolean);
                    const sheetImg = col.sheet === 0 ? fire0 : fire1;
                    return (
                      <button
                        key={c}
                        className={`cell${isCurrent ? " atlas-saved" : ""}${isSug ? " atlas-time" : ""}${isAlt ? " atlas-alt" : ""}${isChosen ? " atlas-candidate" : ""}${isShown ? " atlas-preview" : ""}`}
                        aria-label={`sheet ${col.sheet}, row ${row}, ${col.mirrored ? "mirrored" : "normal"}${states.length ? `; ${states.join(", ")}` : ""}`}
                        aria-pressed={isChosen}
                        title={`sheet ${col.sheet} · row ${row} · ${col.mirrored ? "mirrored" : "normal"}${states.length ? `\n${states.join(" · ")}` : ""}\nClick: select candidate · Ctrl+click: preview only`}
                        onClick={(e) => onCell(c, row, e.ctrlKey)}
                        tabIndex={-1}
                      >
                        <span
                          className="sprite"
                          style={{
                            backgroundImage: `url(${sheetImg})`,
                            backgroundSize: `${CELL}px ${CELL * ROWS}px`,
                            backgroundPosition: `0 -${row * CELL}px`,
                            transform: col.mirrored ? "scaleX(-1)" : undefined,
                          }}
                        />
                        <span className="atlas-badges" aria-hidden="true">
                          {isChosen && <span className="candidate-state">C</span>}
                          {isShown && <span className="preview-state">P</span>}
                          {isSug && <span className="guess-state">T</span>}
                          {isAlt && <span className="guess-state">T?</span>}
                          {isCurrent && <span className="saved-state">✓</span>}
                        </span>
                      </button>
                    );
                  })}
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
