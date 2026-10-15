import { useRef, type KeyboardEvent, type PointerEvent } from "react";
import { dragSize, keyboardSize } from "../lib/layout";

interface Props {
  /** "col": a vertical bar that changes a width; "row": a horizontal bar that changes a height */
  direction: "col" | "row";
  /** the size being controlled, px */
  value: number;
  min: number;
  /** read when a drag starts, so the limit can follow the window size */
  getMax: () => number;
  /** -1 when the controlled pane lies on the far side of the bar (dragging right shrinks it) */
  sign?: 1 | -1;
  label: string;
  onChange: (size: number) => void;
  onReset: () => void;
}

/** A draggable, keyboard-operable bar between two panes. Double-click resets the size. */
export function Splitter({ direction, value, min, getMax, sign = 1, label, onChange, onReset }: Props) {
  const drag = useRef<{ start: number; from: number; max: number } | null>(null);
  const axis = (e: PointerEvent) => (direction === "col" ? e.clientX : e.clientY);

  function down(e: PointerEvent<HTMLDivElement>) {
    if (e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { start: axis(e), from: value, max: getMax() };
    e.preventDefault();
  }
  function move(e: PointerEvent<HTMLDivElement>) {
    const d = drag.current;
    if (d) onChange(dragSize(d.from, axis(e) - d.start, sign, min, d.max));
  }
  function up() {
    drag.current = null;
  }
  function key(e: KeyboardEvent<HTMLDivElement>) {
    const next = keyboardSize(value, e.key, e.shiftKey, min, getMax(), sign);
    if (next === null) return;
    e.preventDefault();
    onChange(next);
  }

  return (
    <div
      className={`splitter ${direction}`}
      role="separator"
      aria-orientation={direction === "col" ? "vertical" : "horizontal"}
      aria-label={label}
      aria-valuenow={value}
      aria-valuemin={min}
      tabIndex={0}
      title={`${label} (drag, arrow keys, double-click to reset)`}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={up}
      onLostPointerCapture={up}
      onDoubleClick={onReset}
      onKeyDown={key}
    />
  );
}
