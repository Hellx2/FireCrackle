import { describe, expect, it, vi } from "vitest";
import type { Backend, View } from "./api";
import { openSession, readSession, SESSION_STORAGE_KEY, writeSession } from "./sessionRestore";

describe("saved video session", () => {
  it("remembers the exact video path and frame", () => {
    const values = new Map<string, string>();
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => { values.set(key, value); },
    };
    const session = { videoPath: "D:\\clips\\火 2026.mkv", frame: 1234 };
    writeSession(storage, session);
    expect(values.has(SESSION_STORAGE_KEY)).toBe(true);
    expect(readSession(storage)).toEqual(session);
  });

  it("ignores absent, corrupt and invalid saved sessions", () => {
    for (const raw of [null, "{", "null", "[]", "{}", '{"videoPath":"","frame":1}',
      '{"videoPath":"clip.mkv","frame":-1}', '{"videoPath":"clip.mkv","frame":1.5}',
      '{"videoPath":"clip.mkv","frame":"1"}']) {
      expect(readSession({ getItem: () => raw })).toBeNull();
    }
  });

  it("reports storage failures so the UI cannot claim resume was saved", () => {
    expect(() => writeSession({ setItem: () => { throw new Error("storage full"); } },
      { videoPath: "clip.mkv", frame: 10 })).toThrow("storage full");
    expect(() => readSession({ getItem: () => { throw new Error("storage blocked"); } })).toThrow("storage blocked");
  });
});

describe("reopening a saved video", () => {
  function backend() {
    const opened = { current: 0, frame_count: 100, labelled_count: 12 } as View;
    const resumed = { ...opened, current: 42 };
    const openVideo = vi.fn(async () => opened);
    const act = vi.fn(async () => resumed);
    return { api: { openVideo, act } as unknown as Backend, openVideo, act, resumed };
  }

  it("loads the existing project before seeking to the saved frame", async () => {
    const b = backend();
    expect(await openSession(b.api, { videoPath: "clip.mkv", frame: 42 })).toBe(b.resumed);
    expect(b.openVideo).toHaveBeenCalledWith("clip.mkv");
    expect(b.act).toHaveBeenCalledWith({ type: "goTo", frame: 42 });
    expect(b.openVideo.mock.invocationCallOrder[0]).toBeLessThan(b.act.mock.invocationCallOrder[0]);
  });

  it("clamps stale positions to the last frame and avoids redundant seeking", async () => {
    const b = backend();
    await openSession(b.api, { videoPath: "clip.mkv", frame: 1000 });
    expect(b.act).toHaveBeenCalledWith({ type: "goTo", frame: 99 });
    b.act.mockClear();
    await openSession(b.api, { videoPath: "clip.mkv", frame: 0 });
    expect(b.act).not.toHaveBeenCalled();
  });

  it("does not seek or overwrite saved state when a video cannot open", async () => {
    const b = backend();
    b.openVideo.mockRejectedValueOnce(new Error("file missing"));
    await expect(openSession(b.api, { videoPath: "clip.mkv", frame: 42 })).rejects.toThrow("file missing");
    expect(b.act).not.toHaveBeenCalled();
  });
});
