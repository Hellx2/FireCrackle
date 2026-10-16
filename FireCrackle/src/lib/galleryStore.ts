import { imageIndex, isPhoto, type GalleryImage } from "./imageGallery";
import type { Label } from "./grid";

export interface SavedGallery { images: GalleryImage[]; index: number; checks: Record<string, Label> }
const DATABASE = "firecrackle-photo-gallery";

function openGallery(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === "undefined") { reject(Error("Local photo storage is unavailable.")); return; }
    const request = indexedDB.open(DATABASE, 1);
    request.onupgradeneeded = () => {
      request.result.createObjectStore("photos", { keyPath: "id" });
      request.result.createObjectStore("state");
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? Error("Could not open local photo storage."));
    request.onblocked = () => reject(Error("Another FireCrackle window is blocking photo storage. Close it and retry."));
  });
}

export async function loadGallery(): Promise<SavedGallery> {
  const db = await openGallery();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(["photos", "state"], "readonly");
    const photos = transaction.objectStore("photos").getAll();
    const state = transaction.objectStore("state").get("gallery");
    transaction.oncomplete = () => {
      db.close();
      const images: GalleryImage[] = [];
      try {
        const records = new Map<string, { id: string; name: string; file: Blob; modified: number }>(photos.result.map(row => [row.id, row]));
        const order: string[] = state.result?.order ?? Array.from(records.keys());
        for (const id of order) {
          const row = records.get(id);
          if (!row || !(row.file instanceof Blob)) continue;
          const file = new File([row.file], row.name, { type: row.file.type, lastModified: row.modified });
          if (isPhoto(file)) images.push({ id, name: row.name, file, url: URL.createObjectURL(file) });
        }
        const checks: Record<string, Label> = {};
        for (const image of images) if (state.result?.checks?.[image.id]) checks[image.id] = state.result.checks[image.id];
        resolve({ images, index: imageIndex(images.findIndex(image => image.id === state.result?.selected), images.length), checks });
      } catch (error) { for (const image of images) URL.revokeObjectURL(image.url); reject(error); }
    };
    transaction.onabort = transaction.onerror = () => { db.close(); reject(transaction.error ?? Error("Could not restore photos.")); };
  });
}

let pending: Promise<void> = Promise.resolve();
/** Serialize atomic updates so rapid add/remove/selection changes cannot restore stale state. */
export function saveGallery(images: GalleryImage[], index: number, checks: Record<string, Label>): Promise<void> {
  const snapshot = [...images], labels = { ...checks };
  const write = pending.catch(() => {}).then(async () => {
    const db = await openGallery();
    return new Promise<void>((resolve, reject) => {
      const transaction = db.transaction(["photos", "state"], "readwrite");
      const photos = transaction.objectStore("photos");
      const existing = photos.getAllKeys();
      existing.onsuccess = () => {
        const oldIds = new Set(existing.result), ids = new Set(snapshot.map(image => image.id));
        for (const id of oldIds) if (!ids.has(String(id))) photos.delete(id);
        // Stored blobs are immutable; moving a selection never rewrites photo bytes.
        for (const image of snapshot) if (!oldIds.has(image.id)) photos.put({ id: image.id, name: image.name, file: image.file, modified: image.file.lastModified });
        transaction.objectStore("state").put({ order: snapshot.map(image => image.id), selected: snapshot[imageIndex(index, snapshot.length)]?.id, checks: labels }, "gallery");
      };
      transaction.oncomplete = () => { db.close(); resolve(); };
      transaction.onabort = transaction.onerror = () => { db.close(); reject(transaction.error ?? Error("Could not save photos.")); };
    });
  });
  pending = write;
  return write;
}
