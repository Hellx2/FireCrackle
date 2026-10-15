import type { View } from "./api";
import { COLUMNS, isFire } from "./grid";

export interface PreviewSprite {
  sheet: 0 | 1;
  row: number;
  mirrored: boolean;
}
export type PreviewMode = "none" | "manual" | "firstPerson";

/** First-person geometry always uses the game's fixed appearance/orientation. */
export function previewSprite(mode: PreviewMode, sprite: PreviewSprite): PreviewSprite | null {
  if (mode === "none" || !Number.isInteger(sprite.row) || sprite.row < 0 || sprite.row >= 32 || ![0, 1].includes(sprite.sheet)) return null;
  return { sheet: mode === "firstPerson" ? 1 : sprite.sheet, row: sprite.row, mirrored: mode === "firstPerson" ? false : sprite.mirrored };
}

export function atlasSprite(col: number, row: number): PreviewSprite | null {
  const column = Number.isInteger(col) ? COLUMNS[col] : null;
  return column ? previewSprite("manual", { ...column, row }) : null;
}

export function selectAtlasCell(ctrlKey: boolean, col: number, row: number, preview: (sprite: PreviewSprite) => boolean, label: (col: number, row: number) => void): void {
  const sprite = atlasSprite(col, row);
  if (ctrlKey && sprite && preview(sprite)) return;
  label(col, row);
}

/** Prefer the destination prediction; an explicit label is a useful fallback. */
export function framePreview(view: Pick<View, "suggestion" | "label">): PreviewSprite | null {
  if (view.suggestion) {
    const sprite = previewSprite("manual", view.suggestion as PreviewSprite);
    if (sprite) return sprite;
  }
  return isFire(view.label) ? previewSprite("manual", view.label.fire as PreviewSprite) : null;
}
