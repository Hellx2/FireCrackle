// The typed contract with the backend (Rust `studio_core::workspace`), plus its two implementations:
// the real one over Tauri's `invoke`, and an in-browser mock (synthetic video) for developing the UI.

import type { UiAction } from "./actions";
import type { PhaseChart } from "./chart";
import type { Label } from "./grid";
import type { ThumbStatus } from "./thumbs";

export interface PickState {
  sheet: number;
  mirrored: boolean;
}

export interface Suggestion {
  sheet: number;
  mirrored: boolean;
  row: number;
  alt_row: number | null;
  from_frame: number;
}

export interface View {
  current: number;
  frame_count: number;
  pts: number;
  fps: number;
  label: Label;
  pick: PickState;
  typed_pending: number | null;
  suggestion: Suggestion | null;
  labelled_count: number;
  flag_count: number;
  warnings: string[];
  save_error: string | null;
}

export type FlagKind = "phase_outlier" | "phase_shift" | "phase_excursion";
export type Severity = "hard" | "soft";

export interface Flag {
  frame: number;
  end_frame: number;
  kind: FlagKind;
  severity: Severity;
  phase_delta: number;
}

export interface Bucket {
  frames: number;
  unset: number;
  fire: number;
  no_fire: number;
  skip: number;
  flag: Severity | null;
}

export interface SettingInfo {
  key: string;
  label: string;
  value: number;
  default: number;
  min: number;
  max: number;
  integer: boolean;
  explanation: string;
}

export interface Backend {
  /** True for the in-browser mock (nothing is saved). */
  readonly mock: boolean;
  /** Ask the user for a video file (a native dialog); null when cancelled or unsupported. */
  pickVideo(): Promise<string | null>;
  /** An optional video passed to the desktop app on launch. */
  startupVideo?(): Promise<string | null>;
  openVideo(path: string): Promise<View>;
  view(): Promise<View>;
  act(action: UiAction): Promise<View>;
  frame(frame: number): Promise<ArrayBuffer>;
  timeline(start: number, end: number, buckets: number): Promise<Bucket[]>;
  flags(): Promise<Flag[]>;
  /** The phase series (at most `maxPoints` points), runs and flags for the Phase tab. */
  phaseChart(maxPoints: number): Promise<PhaseChart>;
  /** Start (or find already made) the timeline thumbnails; they are generated in the background. */
  thumbsStart(): Promise<ThumbStatus>;
  thumbsStatus(): Promise<ThumbStatus>;
  /** For each wanted frame, the frame of the nearest stored thumbnail (null while there are none). */
  thumbNearest(frames: number[]): Promise<Array<number | null>>;
  thumb(frame: number): Promise<ArrayBuffer>;
  settingsDescribe(): Promise<SettingInfo[]>;
  settingsSet(key: string, value: number): Promise<View>;
}

export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** The real backend when running inside Tauri, otherwise the mock. */
export async function createBackend(): Promise<Backend> {
  if (!inTauri()) {
    const { createMockBackend } = await import("./mock");
    return createMockBackend();
  }
  const { invoke } = await import("@tauri-apps/api/core");
  const { open } = await import("@tauri-apps/plugin-dialog");
  return {
    mock: false,
    startupVideo: () => invoke<string | null>("startup_video"),
    async pickVideo() {
      const picked = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "Video", extensions: ["mkv", "mp4", "webm", "mov", "avi", "m4v"] }],
      });
      return typeof picked === "string" ? picked : null;
    },
    openVideo: (path) => invoke<View>("open_video", { path }),
    view: () => invoke<View>("view"),
    act: (action) => invoke<View>("act", { action }),
    frame: (frame) => invoke<ArrayBuffer>("frame", { frame }),
    timeline: (start, end, buckets) => invoke<Bucket[]>("timeline", { start, end, buckets }),
    flags: () => invoke<Flag[]>("flags"),
    phaseChart: (maxPoints) => invoke<PhaseChart>("phase_chart", { maxPoints }),
    thumbsStart: () => invoke<ThumbStatus>("thumbs_start"),
    thumbsStatus: () => invoke<ThumbStatus>("thumbs_status"),
    thumbNearest: (frames) => invoke<Array<number | null>>("thumb_nearest", { frames }),
    thumb: (frame) => invoke<ArrayBuffer>("thumb", { frame }),
    settingsDescribe: () => invoke<SettingInfo[]>("settings_describe"),
    settingsSet: (key, value) => invoke<View>("settings_set", { key, value }),
  };
}

/** Read a number setting from the described list, with a fallback while it is not loaded yet. */
export function settingValue(settings: SettingInfo[], key: string, fallback: number): number {
  return settings.find((s) => s.key === key)?.value ?? fallback;
}
