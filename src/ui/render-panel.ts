import { homography } from "../engine/index.ts";
import { $, $input, setPressed } from "./dom.ts";
import { activePts } from "./edits.ts";
import { EXTENSIONS, FRAME_FULL_BELOW, gapPercent, outputSize } from "./output-size.ts";
import { cardEls } from "./points.ts";
import { S } from "./state.ts";

function syncSlider(input: HTMLInputElement, value: number) {
  input.value = String(value);
  const min = Number(input.min);
  const max = Number(input.max);
  input.style.setProperty("--fill", ((value - min) / (max - min)) * 100 + "%");
  input.closest(".slider__row")!.querySelector("output")!.textContent = String(value);
}

/** Draws the status bar and the side panel. */
export function renderPanel() {
  const { W, H } = S;
  const placing = S.tool === "place";
  const compare = S.compare && !placing;
  const M = homography(S.goodSrc, S.good);
  const pts = activePts();

  // Status bar
  const gap = gapPercent(M, W, H);
  const full = gap < FRAME_FULL_BELOW;
  let tag: [string, string, string];
  if (S.invalid) tag = ["error", "error", "Points crossed"];
  else if (placing) tag = ["neutral", "location_on", "4 points placed"];
  else if (full) tag = ["success", "check_circle", "Frame full"];
  else tag = ["warning", "warning", "Gaps " + gap.toFixed(1) + "%"];
  $("statusTag").className = "tag tag--" + tag[0];
  $("statusIcon").textContent = tag[1];
  $("statusLabel").textContent = tag[2];
  $("statusFile").textContent = S.fileName;
  $("statusSize").textContent = W + " × " + H + " px";
  $("statusHint").textContent = S.invalid
    ? "Move point " + (S.selected + 1) + " back so the 4 points stay a four-sided shape, or ease the camera back."
    : compare
      ? "Drag the divider to compare the shifted view with the original."
      : placing
        ? "Put each point on a straight feature, like a tile or window corner, and keep them far apart. Arrow keys nudge 1 px, Shift + arrow 10 px."
        : "Drag a point to shift the view, or drag inside the shape to slide it. Arrow keys nudge 1 px, Shift + arrow 10 px.";

  // Panel
  $("tabShift").setAttribute("aria-selected", String(S.tab === 0));
  $("tabExport").setAttribute("aria-selected", String(S.tab === 1));
  $("tabShift").tabIndex = S.tab === 0 ? 0 : -1;
  $("tabExport").tabIndex = S.tab === 1 ? 0 : -1;
  $("panelShift").hidden = S.tab !== 0;
  $("panelExport").hidden = S.tab !== 1;
  $("pointsCaption").textContent = placing ? "Pixels on the photo" : "Pixels in the frame";

  [0, 1, 2, 3].forEach((i) => {
    cardEls[i].classList.toggle("is-selected", S.selected === i);
    for (const axis of ["x", "y"] as const) {
      const input = $input("p" + i + axis);
      if (document.activeElement !== input) input.value = String(Math.round(pts[i][axis]) || 0);
    }
  });

  document.querySelectorAll<HTMLInputElement>("[data-cam]").forEach((input) => {
    const key = input.dataset.cam as keyof typeof S.cam;
    syncSlider(input, Math.round(S.cam[key]) || 0);
  });

  // Export panel
  const ext = EXTENSIONS[S.format];
  const noAlpha = S.format === "jpeg";
  const out = outputSize(M, W, H, S.size, S.empty);
  const nameInput = $input("exportName");
  if (document.activeElement !== nameInput) nameInput.value = S.exportName;
  $("exportExt").textContent = ext + " is added on export";
  document.querySelectorAll<HTMLElement>("[data-format]").forEach((b) => setPressed(b, b.dataset.format === S.format));
  $<HTMLSelectElement>("exportSize").value = S.size;
  $("qualityRow").hidden = !(S.format === "jpeg" || S.format === "webp");
  syncSlider($input("exportQuality"), S.quality);
  document.querySelectorAll<HTMLInputElement>('input[name="empty"]').forEach((r) => {
    r.checked = S.empty === r.value;
    r.disabled = r.value === "clear" && noAlpha;
  });
  $("emptyNote").textContent = noAlpha ? "Transparency needs PNG, TIFF or WebP." : "";
  $("summaryOutput").textContent = out ? out.width + " × " + out.height + " px" : "Nothing to crop to";
  $("summaryGaps").textContent = full ? "None" : gap.toFixed(1) + "%";

  $<HTMLButtonElement>("exportBtn").disabled = S.exporting;
  $("exportProgress").hidden = !S.exporting;
  if (S.exporting) {
    const percent = Math.round(S.exportProgress * 100);
    $("exportProgressLabel").textContent = "Exporting " + percent + "%";
    $("exportProgressFill").style.width = percent + "%";
    $("exportProgressBar").setAttribute("aria-valuenow", String(percent));
  }
  $("exportError").hidden = !S.exportError;
  $("exportErrorMessage").textContent = S.exportError;
  $("exportNotice").hidden = !S.exported;
  if (S.exported) {
    $("exportMessage").textContent = "Saved " + S.exported.fileName + " at " + S.exported.width + " × " + S.exported.height + " px.";
  }
}
