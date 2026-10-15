// Hotkeys for the Label tab: a pure mapping from a key event to an action. No DOM access here, so it is
// unit-testable; the React layer feeds it the event and executes the action.

export type Action =
  | { type: "step"; frames: number }
  | { type: "bigStep"; dir: 1 | -1 }
  | { type: "home" }
  | { type: "end" }
  | { type: "playPause" }
  | { type: "shuttle"; dir: -1 | 0 | 1 }
  | { type: "toggleSheet" }
  | { type: "toggleMirror" }
  | { type: "digit"; digit: number }
  | { type: "acceptSuggestion" }
  | { type: "clear" }
  | { type: "noFire" }
  | { type: "skip" }
  | { type: "nextUnlabelled"; dir: 1 | -1 }
  | { type: "nextFlag"; dir: 1 | -1 }
  | { type: "undo" }
  | { type: "cancelTyping" };

/** What the handler needs to know about a keyboard event. */
export interface KeyInfo {
  key: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  repeat: boolean;
  /** The event target is a text field / editable element: hotkeys must not fire. */
  targetEditable: boolean;
}

/** Canonical binding ids: `Ctrl+Alt+Shift+<key>`, letters lower case, space as `Space`. */
export type Bindings = Readonly<Record<string, Action>>;

function buildDefaults(): Bindings {
  const b: Record<string, Action> = {
    ArrowRight: { type: "step", frames: 1 },
    ArrowLeft: { type: "step", frames: -1 },
    "Shift+ArrowRight": { type: "bigStep", dir: 1 },
    "Shift+ArrowLeft": { type: "bigStep", dir: -1 },
    Home: { type: "home" },
    End: { type: "end" },
    Space: { type: "playPause" },
    j: { type: "shuttle", dir: -1 },
    k: { type: "shuttle", dir: 0 },
    l: { type: "shuttle", dir: 1 },
    s: { type: "toggleSheet" },
    m: { type: "toggleMirror" },
    Enter: { type: "acceptSuggestion" },
    Backspace: { type: "clear" },
    x: { type: "noFire" },
    "Shift+x": { type: "skip" },
    n: { type: "nextUnlabelled", dir: 1 },
    "Shift+n": { type: "nextUnlabelled", dir: -1 },
    f: { type: "nextFlag", dir: 1 },
    "Shift+f": { type: "nextFlag", dir: -1 },
    "Ctrl+z": { type: "undo" },
    Escape: { type: "cancelTyping" },
  };
  for (let d = 0; d <= 9; d++) b[String(d)] = { type: "digit", digit: d };
  return b;
}

export const DEFAULT_BINDINGS: Bindings = buildDefaults();

/** Actions that may fire again while the key is held (everything else must not flood). */
export const REPEATABLE: ReadonlySet<Action["type"]> = new Set<Action["type"]>(["step", "bigStep"]);

/** The canonical id of a key event, e.g. `Shift+ArrowRight`, `Ctrl+z`, `Space`. Meta counts as Ctrl. */
export function bindingId(e: KeyInfo): string {
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  parts.push(e.key === " " ? "Space" : e.key.length === 1 ? e.key.toLowerCase() : e.key);
  return parts.join("+");
}

/** The action for a key event, or null (not bound, typing in a text field, or an unwanted auto-repeat). */
export function actionFor(e: KeyInfo, bindings: Bindings = DEFAULT_BINDINGS): Action | null {
  if (e.targetEditable) return null;
  const id = bindingId(e);
  if (!Object.prototype.hasOwnProperty.call(bindings, id)) return null;
  const action = bindings[id];
  if (e.repeat && !REPEATABLE.has(action.type)) return null;
  return action;
}
