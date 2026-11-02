import { $ } from "./dom.ts";
import { commands, type CommandId } from "./commands.ts";
import { startDrag } from "./drag.ts";
import { poseTo, setCam } from "./edits.ts";
import { flatCam, type Camera } from "../engine/index.ts";
import { cancelExport, runExport } from "./export-panel.ts";
import { clearMessage } from "./messages.ts";
import { requestSample } from "./open-photo.ts";
import { clamp } from "./dom.ts";
import { S, set, type EmptyAreas, type ExportFormat, type Tool } from "./state.ts";

const BUTTONS: Record<string, CommandId> = {
  openBtn: "open",
  startOpenBtn: "open",
  goExportBtn: "export",
  themeBtn: "theme",
  undoBtn: "undo",
  redoBtn: "redo",
  resetBtn: "reset",
  compareBtn: "compare",
  meshBtn: "mesh",
  gridBtn: "grid",
  zoomOutBtn: "zoom-out",
  zoomInBtn: "zoom-in",
  zoomFitBtn: "fit",
};

export function wireControls() {
  for (const [id, command] of Object.entries(BUTTONS)) {
    $(id).addEventListener("click", () => commands[command]());
  }
  $("sampleBtn").addEventListener("click", () => void requestSample());
  $("appMessageClose").addEventListener("click", clearMessage);

  document.querySelectorAll<HTMLElement>("[data-tool]").forEach((b) =>
    b.addEventListener("click", () => set({ tool: b.dataset.tool as Tool, compare: false })),
  );

  $("quadGrab").addEventListener("pointerdown", (e) => startDrag(e, "quad"));
  $("divider").addEventListener("pointerdown", (e) => startDrag(e, "split"));
  $("divider").addEventListener("keydown", (e) => {
    const d = ({ ArrowLeft: -0.02, ArrowRight: 0.02 } as Record<string, number>)[e.key];
    if (!d) return;
    e.preventDefault();
    set({ split: clamp(S.split + d, 0, 1) });
  });

  const tabs = [$("tabShift"), $("tabExport")];
  tabs.forEach((tab, i) => {
    tab.addEventListener("click", () => set({ tab: i }));
    tab.addEventListener("keydown", (e) => {
      if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
      e.preventDefault();
      const next = 1 - i;
      set({ tab: next });
      tabs[next].focus();
    });
  });

  document.querySelectorAll<HTMLInputElement>("[data-cam]").forEach((input) => {
    input.addEventListener("input", () => setCam(input.dataset.cam as keyof Camera, Number(input.value)));
  });
  document.querySelectorAll<HTMLElement>("[data-preset]").forEach((chip) => {
    chip.addEventListener("click", () => poseTo({ ...flatCam(), ...JSON.parse(chip.dataset.preset!) }));
  });

  $("exportName").addEventListener("input", (e) => set({ exportName: (e.target as HTMLInputElement).value, exported: null }));
  document.querySelectorAll<HTMLElement>("[data-format]").forEach((b) =>
    b.addEventListener("click", () => {
      const format = b.dataset.format as ExportFormat;
      set({ format, exported: null, empty: format === "jpeg" && S.empty === "clear" ? "crop" : S.empty });
    }),
  );
  $("exportSize").addEventListener("change", (e) => set({ size: (e.target as HTMLSelectElement).value, exported: null }));
  $("exportQuality").addEventListener("input", (e) => set({ quality: Number((e.target as HTMLInputElement).value), exported: null }));
  document.querySelectorAll<HTMLInputElement>('input[name="empty"]').forEach((r) =>
    r.addEventListener("change", () => set({ empty: r.value as EmptyAreas, exported: null })),
  );

  $("exportBtn").addEventListener("click", () => void runExport());
  $("exportCancelBtn").addEventListener("click", cancelExport);

  new ResizeObserver(() => {
    const st = $("stage");
    if (st.clientWidth > 0) set({ stageW: st.clientWidth, stageH: st.clientHeight });
  }).observe($("stage"));
}
