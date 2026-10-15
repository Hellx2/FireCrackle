import { useEffect, useState } from "react";
import "./App.css";
import { FlagsPanel } from "./components/FlagsPanel";
import { PhaseTab } from "./components/PhaseTab";
import { Picker } from "./components/Picker";
import { SettingsPanel } from "./components/SettingsPanel";
import { Timeline } from "./components/Timeline";
import { VideoPane } from "./components/VideoPane";
import { useWorkspace } from "./hooks/useWorkspace";
import { STOPPED, keyInfo, nextPlayback, playbackTick, toUiAction, type Playback } from "./lib/actions";
import { createBackend, settingValue, type Backend } from "./lib/api";
import { actionFor } from "./lib/keymap";

const KEY_HELP: Array<[string, string]> = [
  ["← →", "previous / next frame (hold to repeat)"],
  ["Shift + ← →", "big step"],
  ["Home / End", "first / last frame"],
  ["Space · L · J · K", "play / faster forward / faster backward / stop"],
  ["0–9", "type a row (12, 31 …); 4–9 label at once"],
  ["Enter", "accept the suggested row"],
  ["S · M", "toggle sheet · toggle mirrored"],
  ["X · Shift+X", "no fire · skip"],
  ["Backspace · Ctrl+Z", "clear · undo"],
  ["N · Shift+N", "next / previous unlabelled"],
  ["F · Shift+F", "next / previous flag"],
  ["Esc", "cancel a half-typed row"],
];

export default function App() {
  const [backend, setBackend] = useState<Backend | null>(null);
  const [startupError, setStartupError] = useState<string | null>(null);
  useEffect(() => {
    createBackend().then(setBackend).catch((e) => setStartupError(String(e)));
  }, []);

  const ws = useWorkspace(backend);
  const { view, act } = ws;
  const [playback, setPlayback] = useState<Playback>(STOPPED);
  const [showSettings, setShowSettings] = useState(false);
  const [showKeys, setShowKeys] = useState(false);
  const [path, setPath] = useState("");
  const [tab, setTab] = useState<"label" | "phase">("label");

  /** Leave the Phase tab at a frame (a flag, or a point of the chart). */
  function jumpToLabel(frame: number) {
    setTab("label");
    void act({ type: "goTo", frame });
  }

  const maxSpeed = settingValue(ws.settings, "label.max_play_speed", 8);
  const minTick = settingValue(ws.settings, "label.min_play_tick_ms", 16);
  const typeTimeout = settingValue(ws.settings, "label.row_type_timeout_ms", 700);

  // hotkeys
  useEffect(() => {
    if (!view || tab !== "label") return;
    const onKey = (e: KeyboardEvent) => {
      const a = actionFor(
        keyInfo({
          key: e.key,
          ctrlKey: e.ctrlKey,
          shiftKey: e.shiftKey,
          altKey: e.altKey,
          metaKey: e.metaKey,
          repeat: e.repeat,
          target: e.target as HTMLElement | null,
        }),
      );
      if (!a) return;
      e.preventDefault();
      const ui = toUiAction(a);
      if (ui) void act(ui);
      else if (a.type === "playPause" || a.type === "shuttle") setPlayback((p) => nextPlayback(p, a, maxSpeed));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [view !== null, tab, act, maxSpeed]); // eslint-disable-line react-hooks/exhaustive-deps

  // playback is a Label-tab thing: it must not keep stepping behind the chart
  useEffect(() => {
    if (tab !== "label") setPlayback(STOPPED);
  }, [tab]);

  // playback: one timer; ticks are skipped while the previous step is still in flight
  const fps = view?.fps ?? 0;
  useEffect(() => {
    const tick = playbackTick(fps, playback, minTick);
    if (!tick) return;
    const id = setInterval(() => {
      if (!ws.busy.current) void act({ type: "step", frames: tick.frames });
    }, tick.intervalMs);
    return () => clearInterval(id);
  }, [fps, playback, minTick, act, ws.busy]);

  // stop at the ends
  useEffect(() => {
    if (!view) return;
    if ((playback.dir > 0 && view.current >= view.frame_count - 1) || (playback.dir < 0 && view.current <= 0)) setPlayback(STOPPED);
  }, [view?.current, view?.frame_count, playback.dir]); // eslint-disable-line react-hooks/exhaustive-deps

  // a single typed digit that could start a two-digit row is committed after the timeout
  useEffect(() => {
    if (view?.typed_pending == null) return;
    const id = setTimeout(() => void act({ type: "flushTyped" }), typeTimeout);
    return () => clearTimeout(id);
  }, [view?.typed_pending, typeTimeout, act]);

  async function chooseVideo() {
    if (!backend) return;
    const picked = await backend.pickVideo();
    if (picked) {
      setPlayback(STOPPED);
      await ws.open(picked);
    }
  }

  if (startupError) return <div className="fatal">Could not start: {startupError}</div>;
  if (!backend) return <div className="fatal">starting…</div>;

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          FireCrackle
          {backend.mock && <span className="tag mock">mock backend</span>}
        </div>
        {view && (
          <nav className="tabs">
            <button className={tab === "label" ? "on" : ""} onClick={() => setTab("label")}>
              Label
            </button>
            <button className={tab === "phase" ? "on" : ""} onClick={() => setTab("phase")}>
              Phase{view.flag_count > 0 && <span className="count">{view.flag_count}</span>}
            </button>
          </nav>
        )}
        <div className="top-actions">
          <button onClick={() => void chooseVideo()} disabled={ws.opening}>
            {ws.opening ? "importing…" : "Open video…"}
          </button>
          {view && (
            <>
              <button onClick={() => void act({ type: "undo" })} title="Ctrl+Z">
                Undo
              </button>
              <button
                className={playback.dir !== 0 ? "on" : ""}
                onClick={() => setPlayback((p) => nextPlayback(p, { type: "playPause" }, maxSpeed))}
                title="Space"
              >
                {playback.dir === 0 ? "▶ Play" : `⏸ ${playback.dir < 0 ? "◀" : "▶"} ${playback.speed}×`}
              </button>
              <button onClick={() => setShowKeys((s) => !s)}>Keys</button>
              <button onClick={() => setShowSettings(true)}>Settings</button>
            </>
          )}
        </div>
      </header>

      {ws.error && (
        <div className="banner error" onClick={ws.clearError}>
          {ws.error} <span className="dismiss">(click to dismiss)</span>
        </div>
      )}
      {view?.save_error && <div className="banner error">⚠ {view.save_error}</div>}
      {view && view.warnings.length > 0 && (
        <details className="banner warn">
          <summary>
            {view.warnings.length} note{view.warnings.length > 1 ? "s" : ""} about this video
          </summary>
          <ul>
            {view.warnings.map((w, i) => (
              <li key={i}>{w}</li>
            ))}
          </ul>
        </details>
      )}

      {!view ? (
        <div className="welcome">
          <h1>Label the fire sprites, frame by frame</h1>
          <p>Open a video. FireCrackle reads its timestamps, you click which sprite is on screen, and the phase check flags what does not add up.</p>
          <button className="big" onClick={() => void chooseVideo()} disabled={ws.opening}>
            {ws.opening ? "importing…" : "Open video…"}
          </button>
          {!backend.mock && (
            <div className="path-entry">
              <input value={path} onChange={(e) => setPath(e.target.value)} placeholder="or paste a path to a video file" />
              <button onClick={() => path && void ws.open(path)} disabled={!path || ws.opening}>
                Open
              </button>
            </div>
          )}
        </div>
      ) : tab === "phase" ? (
        <PhaseTab backend={backend} view={view} version={ws.version} onJump={jumpToLabel} />
      ) : (
        <main className="workarea">
          <section className="left">
            <VideoPane backend={backend} view={view} onError={(m) => console.error(m)} />
            <Timeline
              backend={backend}
              view={view}
              flags={ws.flags}
              version={ws.version}
              settings={ws.settings}
              onSeek={(frame) => void act({ type: "goTo", frame })}
            />
            <FlagsPanel flags={ws.flags} fps={view.fps} current={view.current} onSeek={(frame) => void act({ type: "goTo", frame })} />
          </section>
          <aside className="right">
            <div className="pick-state">
              <button className={view.pick.sheet === 1 ? "on" : ""} onClick={() => void act({ type: "toggleSheet" })} title="S">
                sheet {view.pick.sheet}
              </button>
              <button className={view.pick.mirrored ? "on" : ""} onClick={() => void act({ type: "toggleMirror" })} title="M">
                {view.pick.mirrored ? "mirrored" : "normal"}
              </button>
              <button onClick={() => void act({ type: "noFire" })} title="X">
                no fire
              </button>
              <button onClick={() => void act({ type: "skip" })} title="Shift+X">
                skip
              </button>
              <button onClick={() => void act({ type: "clear" })} title="Backspace">
                clear
              </button>
            </div>
            <Picker view={view} onCell={(col, row) => void act({ type: "clickCell", col, row })} />
          </aside>
        </main>
      )}

      {showKeys && (
        <div className="keys">
          {KEY_HELP.map(([k, d]) => (
            <div key={k}>
              <kbd>{k}</kbd> {d}
            </div>
          ))}
        </div>
      )}
      {showSettings && <SettingsPanel settings={ws.settings} onSet={ws.setSetting} onClose={() => setShowSettings(false)} />}
    </div>
  );
}
