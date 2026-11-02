import { flatCam } from "../engine/index.ts";
import { poseTo, redo, undo } from "./edits.ts";
import { requestOpen } from "./open-photo.ts";
import { S, set } from "./state.ts";
import { toggleTheme } from "./theme.ts";

export type CommandId =
  | "open"
  | "export"
  | "undo"
  | "redo"
  | "reset"
  | "fit"
  | "zoom-in"
  | "zoom-out"
  | "compare"
  | "mesh"
  | "grid"
  | "theme";

const inEditor = (run: () => void) => () => {
  if (S.view === "editor") run();
};

/** The code behind each toolbar button and menu item. Both call these, so they cannot drift apart. */
export const commands: Record<CommandId, () => void> = {
  open: () => void requestOpen(),
  export: inEditor(() => set({ tab: 1 })),
  undo: inEditor(undo),
  redo: inEditor(redo),
  reset: inEditor(() => poseTo(flatCam())),
  fit: inEditor(() => set({ zoom: 1 })),
  "zoom-in": inEditor(() => set({ zoom: Math.min(4, S.zoom + 0.25) })),
  "zoom-out": inEditor(() => set({ zoom: Math.max(0.5, S.zoom - 0.25) })),
  compare: inEditor(() => {
    if (S.tool !== "place") set({ compare: !S.compare });
  }),
  mesh: inEditor(() => set({ mesh: !S.mesh })),
  grid: inEditor(() => set({ grid: !S.grid })),
  theme: toggleTheme,
};

const isTextField = (el: Element | null) =>
  el instanceof HTMLInputElement && el.type === "text";

/** Is a text field focused? It then keeps its own undo and redo. */
export const textFieldFocused = () => isTextField(document.activeElement);

/** A menu shortcut reaches the app before the webview, so hand undo and redo back to a focused field. */
export function runFromMenu(id: string) {
  if ((id === "undo" || id === "redo") && textFieldFocused()) {
    document.execCommand(id);
    return;
  }
  if (id in commands) commands[id as CommandId]();
}
