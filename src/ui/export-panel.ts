import * as api from "../native/api.ts";
import { markSaved } from "./edits.ts";
import { S, set } from "./state.ts";

let started = false;

/** Runs the export from the panel's settings. Uses the last good shape, which is what is on screen. */
export async function runExport() {
  if (S.view !== "editor" || S.exporting || started) return;
  started = true;
  set({ exportError: "", exported: null });
  try {
    const path = await api.chooseExportPath(S.exportName, S.format);
    if (!path) return;
    set({ exporting: true, exportProgress: 0 });
    const src = S.goodSrc;
    const dst = S.good;
    const result = await api.exportPhoto({
      src,
      dst,
      path,
      options: {
        format: S.format,
        longEdge: S.size === "original" ? null : Number(S.size),
        quality: S.quality,
        empty: S.empty,
      },
    });
    // Only this shape is saved. If it was edited during the export, it is still unsaved.
    set({ exported: result });
    if (S.goodSrc === src && S.good === dst) markSaved();
  } catch (e) {
    if (!api.isCancelled(e)) set({ exportError: api.errorMessage(e) });
  } finally {
    started = false;
    set({ exporting: false });
  }
}

export function cancelExport() {
  void api.cancelExport();
}
