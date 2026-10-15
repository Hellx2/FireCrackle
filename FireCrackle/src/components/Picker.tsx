import { useEffect, useRef, type CSSProperties } from "react";
import fire0 from "../assets/fire_0.png";
import fire1 from "../assets/fire_1.png";
import { COLUMNS, ROWS, columnHeader, labelToCell, suggestionCells } from "../lib/grid";
import type { View } from "../lib/api";

interface Props {
  view: View;
  /** sprite size in px; the 16 px sprites are shown scaled */
  cell: number;
  onCell: (col: number, row: number) => void;
}

/** The 4 x 32 sprite grid: click a cell to label the current frame with it. */
export function Picker({ view, cell: CELL, onCell }: Props) {
  const current = labelToCell(view.label);
  const sug = view.suggestion ? suggestionCells(view.suggestion) : { primary: null, alt: null };
  const targetRef = useRef<HTMLDivElement | null>(null);

  const focusRow = sug.primary?.row ?? current?.row ?? null;
  useEffect(() => {
    targetRef.current?.scrollIntoView({ block: "nearest" });
  }, [focusRow]);

  return (
    <div className="picker" style={{ "--cell": `${CELL}px` } as CSSProperties}>
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
        {Array.from({ length: ROWS }, (_, row) => (
          <div key={row} className="picker-row" ref={row === focusRow ? targetRef : undefined}>
            <div className="gutter">{row}</div>
            {COLUMNS.map((col, c) => {
              const isCurrent = current?.col === c && current.row === row;
              const isSug = sug.primary?.col === c && sug.primary.row === row;
              const isAlt = sug.alt?.col === c && sug.alt.row === row;
              const sheetImg = col.sheet === 0 ? fire0 : fire1;
              return (
                <button
                  key={c}
                  className={`cell${isCurrent ? " current" : ""}${isSug ? " suggest" : ""}${isAlt ? " alt" : ""}`}
                  title={`sheet ${col.sheet} · row ${row} · ${col.mirrored ? "mirrored" : "normal"}`}
                  onClick={() => onCell(c, row)}
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
                </button>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}
