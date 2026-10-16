/** Keep image tool geometry/settings separate from the video workspace. */
export function memoryStorage(): Pick<Storage, "getItem" | "setItem"> {
  const values = new Map<string, string>();
  return { getItem: key => values.get(key) ?? null, setItem: (key, value) => { values.set(key, value); } };
}

export function scopedStorage(scope: string, storage?: Pick<Storage, "getItem" | "setItem">): Pick<Storage, "getItem" | "setItem"> {
  const prefix = `firecrackle.scope:${encodeURIComponent(scope)}:`;
  return {
    getItem: key => (storage ?? localStorage).getItem(prefix + key),
    setItem: (key, value) => (storage ?? localStorage).setItem(prefix + key, value),
  };
}
