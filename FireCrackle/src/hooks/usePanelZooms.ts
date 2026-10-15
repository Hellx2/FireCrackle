import { useCallback, useEffect, useState } from "react";
import { DEFAULT_ZOOMS, ZOOM_STORAGE_KEY, parseZooms, serializeZooms, zoomPanel, type PanelZooms } from "../lib/zoom";

function load(): PanelZooms {
  try {
    return parseZooms(localStorage.getItem(ZOOM_STORAGE_KEY));
  } catch {
    return DEFAULT_ZOOMS;
  }
}

/** Each panel's own zoom (ctrl+wheel), remembered between runs. */
export function usePanelZooms() {
  const [zooms, setZooms] = useState<PanelZooms>(load);

  useEffect(() => {
    try {
      localStorage.setItem(ZOOM_STORAGE_KEY, serializeZooms(zooms));
    } catch {
      /* not remembered */
    }
  }, [zooms]);

  const wheel = useCallback((key: keyof PanelZooms, deltaY: number) => setZooms((z) => ({ ...z, [key]: zoomPanel(z[key], deltaY) })), []);
  const reset = useCallback((key: keyof PanelZooms) => setZooms((z) => ({ ...z, [key]: DEFAULT_ZOOMS[key] })), []);
  return { zooms, wheel, reset };
}
