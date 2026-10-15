import { useCallback, useEffect, useRef, useState } from "react";
import type { UiAction } from "../lib/actions";
import type { Backend, Flag, SettingInfo, View } from "../lib/api";
import { openSession, readSession, writeSession } from "../lib/sessionRestore";

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
  "manualStep",
  "manualBigStep",
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
  saveStatus: string;
  save: () => Promise<void>;
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
  const videoPath = useRef<string | null>(null);
  const restoredBackend = useRef<Backend | null>(null);
  const [saveStatus, setSaveStatus] = useState("Autosave on");

  const remember = useCallback((v: View) => {
    if (!backend || backend.mock || !videoPath.current) return;
    try {
      writeSession(localStorage, { videoPath: videoPath.current, frame: v.current });
      setSaveStatus(v.save_error ? "Save failed" : "Autosaved");
    } catch (e) {
      setSaveStatus("Resume unavailable");
      setError(`Labels still autosave, but the last video and frame could not be remembered: ${messageOf(e)}`);
    }
  }, [backend]);

  const open = useCallback(
    async (path: string, frame = 0) => {
      if (!backend) return;
      if (busy.current) return;
      busy.current = true;
      setOpening(true);
      try {
        if (videoPath.current) {
          const previous = await backend.act({ type: "flushTyped" });
          if (previous.save_error) throw new Error(previous.save_error);
        }
        const v = await openSession(backend, { videoPath: path, frame });
        videoPath.current = path;
        setView(v);
        setError(null);
        remember(v);
        setSettings(await backend.settingsDescribe());
        setVersion((x) => x + 1);
      } catch (e) {
        setError(messageOf(e));
      } finally {
        busy.current = false;
        setOpening(false);
      }
    },
    [backend, remember],
  );

  // The ref survives StrictMode's effect replay, so a video is restored once.
  useEffect(() => {
    if (!backend || backend.mock || restoredBackend.current === backend) return;
    restoredBackend.current = backend;
    setOpening(true);
    void (async () => {
      try {
        const launchPath = await backend.startupVideo?.();
        const remembered = readSession(localStorage);
        const saved = launchPath && launchPath !== remembered?.videoPath ? { videoPath: launchPath, frame: 0 } : remembered;
        if (saved) await open(saved.videoPath, saved.frame);
      } catch (e) {
        setError(`Could not restore the last session: ${messageOf(e)}. You can open a video manually.`);
      } finally {
        setOpening(false);
      }
    })();
  }, [backend, open]);

  const act = useCallback(
    async (a: UiAction) => {
      if (!backend || busy.current) return;
      busy.current = true;
      try {
        const v = await backend.act(a);
        setView(v);
        remember(v);
        if (MUTATING.has(a.type)) setVersion((x) => x + 1);
      } catch (e) {
        const m = messageOf(e);
        if (!m.includes("NoSuggestion")) setError(m); // pressing Enter with nothing to accept is not an error
      } finally {
        busy.current = false;
      }
    },
    [backend, remember],
  );

  const save = useCallback(async () => {
    if (!backend || backend.mock || busy.current || !videoPath.current) return;
    busy.current = true;
    setSaveStatus("Saving…");
    try {
      // Flush a half-typed row and retry any label writes that failed earlier.
      const v = await backend.act({ type: "flushTyped" });
      setView(v);
      setVersion((x) => x + 1);
      remember(v);
      if (v.save_error) setError(v.save_error);
    } catch (e) {
      setSaveStatus("Save failed");
      setError(messageOf(e));
    } finally {
      busy.current = false;
    }
  }, [backend, remember]);

  useEffect(() => {
    if (!view?.save_error) return;
    const timer = setInterval(() => void save(), 5000);
    return () => clearInterval(timer);
  }, [view?.save_error, save]);

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

  return { view, flags, settings, error, version, opening, saveStatus, save, busy, open, act, setSetting, clearError: () => setError(null) };
}
