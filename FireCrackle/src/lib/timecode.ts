/** `HH:MM:SS|FF`: the time of a frame, with FF the frame number inside that second (00 .. fps-1). */
export function formatTimecode(seconds: number, fps: number): string {
  const rate = Number.isFinite(fps) && fps >= 1 ? Math.round(fps) : 1;
  const t = Number.isFinite(seconds) ? Math.max(0, seconds) : 0;
  const frames = Math.round(t * rate);
  const whole = Math.floor(frames / rate);
  const ff = frames % rate;
  const h = Math.floor(whole / 3600);
  const m = Math.floor((whole % 3600) / 60);
  const s = whole % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(h)}:${pad(m)}:${pad(s)}|${pad(ff)}`;
}
