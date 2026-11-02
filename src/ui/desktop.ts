import * as api from "../native/api.ts";
import { onMenuCommand } from "../native/menu.ts";
import { runFromMenu, textFieldFocused } from "./commands.ts";
import { $ } from "./dom.ts";
import { isDirty } from "./edits.ts";
import { openDropped } from "./open-photo.ts";
import { S, set } from "./state.ts";

let lastSent = "";

/** Tells the native side about the window title, menu states and unexported work, when they change. */
export function syncDesktop() {
  const editing = S.view === "editor";
  const placing = S.tool === "place";
  const fieldUndo = editing && textFieldFocused();
  const ui: api.UiState = {
    title: editing ? S.fileName + " - Deinon" : "Deinon",
    dirty: isDirty(),
    photoOpen: editing,
    canUndo: fieldUndo || (editing && S.past.length > 0),
    canRedo: fieldUndo || (editing && S.future.length > 0),
    canCompare: editing && !placing,
    compare: editing && S.compare && !placing,
    mesh: S.mesh,
    grid: S.grid,
  };
  const text = JSON.stringify(ui);
  if (text === lastSent) return;
  lastSent = text;
  void api.setUiState(ui).catch((e) => console.error(e));
}

const dropZone = () => (S.view === "start" ? $("dropzone") : $("stage"));

function highlight(on: boolean) {
  $("dropzone").classList.remove("stage--drop");
  $("stage").classList.remove("stage--drop");
  if (on) dropZone().classList.add("stage--drop");
}

export function wireDesktop() {
  void onMenuCommand(runFromMenu);
  void api.onExportProgress((fraction) => set({ exportProgress: fraction }));
  void api.onFileDrop(({ kind, paths }) => {
    if (kind === "enter" || kind === "over") highlight(true);
    else highlight(false);
    if (kind === "drop") void openDropped(paths);
  });

  // Undo and redo stay available in the menu while a text field can use them.
  document.addEventListener("focusin", syncDesktop);
  document.addEventListener("focusout", () => queueMicrotask(syncDesktop));

  if (!import.meta.env.DEV) {
    document.addEventListener("contextmenu", (e) => {
      // Text fields keep theirs, for copy and paste.
      if (!(e.target instanceof HTMLInputElement)) e.preventDefault();
    });
  }
}
