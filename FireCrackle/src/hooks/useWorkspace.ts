import { useCallback, useEffect, useRef, useState } from "react";
import type { UiAction } from "../lib/actions";
import type { Backend, Flag, SettingInfo, View } from "../lib/api";

/** Actions that change labels: after them the timeline lanes and the flag list must be refreshed. */
const MUTATING = new Set<UiAction["type"]>([
  "clickCell",
  "digit",
  "flushTyped",
  "clear",
  "noFire",
  "skip",
  "undo",
  "acceptSuggestion",
  "fillRange",
]);

export function messageOf(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

export interface WorkspaceState {
  view: View | null;
  flags: Flag[];
  settings: SettingInfo[];
  error: string | null;
  /** Increments whenever labels changed. */
  version: number;
  opening: boolean;
  /** True while an action is in flight (playback skips ticks instead of queueing them). */
  busy: React.MutableRefObject<boolean>;
  open: (path: string) => Promise<void>;
  act: (a: UiAction) => Promise<void>;
  setSetting: (key: string, value: number) => Promise<string | null>;
  clearError: () => void;
}

export function useWorkspace(backend: Backend | null): WorkspaceState {
  const [view, setView] = useState<View | null>(null);
  const [flags, setFlags] = useState<Flag[]>([]);
  const [settings, setSettings] = useState<SettingInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [version, setVersion] = useState(0);
  const [opening, setOpening] = useState(false);
  const busy = useRef(false);
  const isOpen = view !== null;

  const open = useCallback(
    async (path: string) => {
      if (!backend) return;
      setOpening(true);
      try {
        const v = await backend.openVideo(path);
        setView(v);
        setSettings(await backend.settingsDescribe());
        setVersion((x) => x + 1);
        setError(null);
      } catch (e) {
        setError(messageOf(e));
      } finally {
        setOpening(false);
      }
    },
    [backend],
  );

  const act = useCallback(
    async (a: UiAction) => {
      if (!backend) return;
      busy.current = true;
      try {
        const v = await backend.act(a);
        setView(v);
        if (MUTATING.has(a.type)) setVersion((x) => x + 1);
      } catch (e) {
        const m = messageOf(e);
        if (!m.includes("NoSuggestion")) setError(m); // pressing Enter with nothing to accept is not an error
      } finally {
        busy.current = false;
      }
    },
    [backend],
  );

  useEffect(() => {
    if (!backend || !isOpen) return;
    let cancelled = false;
    backend
      .flags()
      .then((f) => !cancelled && setFlags(f))
      .catch((e) => !cancelled && setError(messageOf(e)));
    return () => {
      cancelled = true;
    };
  }, [backend, isOpen, version]);

  const setSetting = useCallback(
    async (key: string, value: number): Promise<string | null> => {
      if (!backend) return "no backend";
      try {
        setView(await backend.settingsSet(key, value));
        setSettings(await backend.settingsDescribe());
        setVersion((x) => x + 1); // phase thresholds change the flags and the chart
        return null;
      } catch (e) {
        return messageOf(e);
      }
    },
    [backend],
  );

  return { view, flags, settings, error, version, opening, busy, open, act, setSetting, clearError: () => setError(null) };
}
