import { useEffect, useRef, type RefObject } from "react";

/**
 * Wheel events on an element, with preventDefault working (React's onWheel is passive).
 * The handler returns true when it used the event, which then does not scroll or browser-zoom.
 */
export function useWheel(ref: RefObject<HTMLElement | null>, handler: (e: WheelEvent) => boolean) {
  const latest = useRef(handler);
  latest.current = handler;
  // the element may mount after the first render (it is only there once a video is open), so look again every render
  const bound = useRef<HTMLElement | null>(null);
  const off = useRef<(() => void) | null>(null);
  useEffect(() => {
    const el = ref.current;
    if (el === bound.current) return;
    off.current?.();
    off.current = null;
    bound.current = el;
    if (!el) return;
    const on = (e: WheelEvent) => {
      if (latest.current(e)) e.preventDefault();
    };
    el.addEventListener("wheel", on, { passive: false });
    off.current = () => el.removeEventListener("wheel", on);
  });
  useEffect(() => () => off.current?.(), []);
}
