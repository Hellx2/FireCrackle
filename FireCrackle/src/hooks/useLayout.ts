import { useCallback, useEffect, useState } from "react";
import { DEFAULT_LAYOUT, LAYOUT_STORAGE_KEY, parseLayout, serializeLayout, type Layout } from "../lib/layout";

function load(): Layout {
  try {
    return parseLayout(localStorage.getItem(LAYOUT_STORAGE_KEY));
  } catch {
    return DEFAULT_LAYOUT; // storage blocked
  }
}

/** Pane sizes, remembered between runs (a failing storage just means they are not remembered). */
export function useLayout() {
  const [layout, setLayout] = useState<Layout>(load);

  useEffect(() => {
    try {
      localStorage.setItem(LAYOUT_STORAGE_KEY, serializeLayout(layout));
    } catch {
      /* not remembered */
    }
  }, [layout]);

  const set = useCallback(<K extends keyof Layout>(key: K, value: Layout[K]) => setLayout((l) => ({ ...l, [key]: value })), []);
  const reset = useCallback(<K extends keyof Layout>(key: K) => setLayout((l) => ({ ...l, [key]: DEFAULT_LAYOUT[key] })), []);
  return { layout, set, reset };
}
