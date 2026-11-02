import { syncDesktop } from "./desktop.ts";
import { $ } from "./dom.ts";
import { renderPanel } from "./render-panel.ts";
import { renderStage } from "./render-stage.ts";
import { S } from "./state.ts";

export function render() {
  const onEditor = S.view === "editor";
  $("startView").hidden = onEditor;
  $("editorView").hidden = !onEditor;
  $("goExportBtn").hidden = !onEditor;
  if (onEditor) {
    renderStage();
    renderPanel();
  }
  syncDesktop();
}
