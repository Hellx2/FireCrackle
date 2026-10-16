import { expect, it } from "vitest";
import { appendImages, clipboardPhotos, imageIndex, imageView, isPhoto } from "./imageGallery";
const photo = (name: string, type = "image/png", size = 12) => ({ name, type, size, lastModified: 1 }) as File;
it("accepts supported photos, rejects videos, and deduplicates reimports without replacing the gallery", () => {
  const first = appendImages([], [photo("a.png"), photo("video.mp4", "video/mp4")], file => `blob:${file.name}`);
  expect(first.images.map(image => image.name)).toEqual(["a.png"]); expect(first.rejected).toBe(1);
  const next = appendImages(first.images, [photo("a.png"), photo("b.jpg", "image/jpeg")], file => `blob:${file.name}`);
  expect(next.images.map(image => image.name)).toEqual(["a.png", "b.jpg"]);
  expect(next.images[0]).toBe(first.images[0]); expect(next.images[1].id).not.toBe(next.images[0].id);
  expect(isPhoto(photo("untagged.webp", ""))).toBe(true);
  expect(isPhoto(photo("bad.png", "video/mp4"))).toBe(false);
});
it("keeps image selection in range after deleting the last photo", () => {
  expect(imageIndex(5, 3)).toBe(2); expect(imageIndex(-1, 3)).toBe(0); expect(imageIndex(0, 0)).toBe(0);
});
it("never invents a time guess or saved label for image checks", () => {
  const view = imageView(2, 4, { sheet: 0, mirrored: true, row: 9 });
  expect(view).toMatchObject({ current: 2, frame_count: 4, label: "unset", suggestion: null, labelled_count: 0, flag_count: 0, pick: { sheet: 0, mirrored: true } });
});
it("reads clipboard photos once from files or falls back to clipboard items, ignoring text", () => {
  const image = photo("paste.png");
  const data = { files: [image], items: [{ kind: "file", getAsFile: () => image }] } as unknown as DataTransfer;
  expect(clipboardPhotos(data)).toEqual([image]);
  expect(clipboardPhotos({ ...data, files: [] } as unknown as DataTransfer)).toEqual([image]);
  expect(clipboardPhotos(null)).toEqual([]);
  expect(clipboardPhotos({ files: [], items: [{ kind: "string", getAsFile: () => null }] } as unknown as DataTransfer)).toEqual([]);
});
