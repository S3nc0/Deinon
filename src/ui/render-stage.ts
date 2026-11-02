import { homography, squareToQuad, toCssMatrix, type Point } from "../engine/index.ts";
import { $, $svg, place, r2, setPressed } from "./dom.ts";
import { activePts } from "./edits.ts";
import { ghostEls, pointEls } from "./points.ts";
import { geom, S } from "./state.ts";


/** Draws the toolbar's state and everything on the stage. */
export function renderStage() {
  const { W, H } = S;
  const placing = S.tool === "place";
  const sw = S.stageW;
  const sh = S.stageH;
  const margin = window.innerWidth < 880 ? 32 : 64;
  const fit = Math.max(0.05, Math.min((sw - margin * 2) / W, (sh - margin * 2) / H));
  const s = fit * S.zoom;
  const fw = W * s;
  const fh = H * s;
  const fx = (sw - fw) / 2;
  const fy = (sh - fh) / 2;
  Object.assign(geom, { fx, fy, fw, fh, s, sw, sh });
  const toStage = (p: Point): Point => ({ x: fx + p.x * s, y: fy + p.y * s });
  const M = homography(S.goodSrc, S.good);
  const compare = S.compare && !placing;

  const stage = $("stage");
  stage.classList.toggle("tool-place", placing);
  stage.classList.toggle("is-invalid", S.invalid);

  // Toolbar
  document.querySelectorAll<HTMLElement>("[data-tool]").forEach((b) => setPressed(b, b.dataset.tool === S.tool));
  $<HTMLButtonElement>("undoBtn").disabled = !S.past.length;
  $<HTMLButtonElement>("redoBtn").disabled = !S.future.length;
  $<HTMLButtonElement>("compareBtn").disabled = placing;
  setPressed($("compareBtn"), compare);
  setPressed($("meshBtn"), S.mesh);
  setPressed($("gridBtn"), S.grid);
  $("zoomLabel").textContent = Math.round(s * 100) + "%";

  // Photo and frame
  place($("checker"), fx, fy, fw, fh);
  const photo = $<HTMLImageElement>("photo");
  if (photo.getAttribute("src") !== S.photo) {
    photo.setAttribute("src", S.photo);
    $("compareImg").setAttribute("src", S.photo);
    $("loupeImg").setAttribute("src", S.photo);
    photo.alt = "Photo being shifted: " + S.fileName;
  }
  place(photo, fx, fy, W, H);
  photo.style.transform = "scale(" + s + ")" + (placing ? "" : " " + toCssMatrix(M));
  place($("mask"), fx, fy, fw, fh);
  place($("frameLabel"), fx, fy - 22);

  // Overlays
  const pts = activePts();
  const stagePts = pts.map(toStage);
  const seg = (a: Point, b: Point) => "M" + r2(a.x) + " " + r2(a.y) + "L" + r2(b.x) + " " + r2(b.y);
  const quad = compare ? "" : "M" + stagePts.map((p) => r2(p.x) + " " + r2(p.y)).join("L") + "Z";
  $svg("edgePath").setAttribute("d", quad);
  $svg("edgeUnder").setAttribute("d", quad);

  let mesh = "";
  if (S.mesh && !S.invalid && !compare) {
    const Q = squareToQuad(pts);
    const at = (u: number, v: number) => {
      const w = Q[6] * u + Q[7] * v + 1;
      return toStage({ x: (Q[0] * u + Q[1] * v + Q[2]) / w, y: (Q[3] * u + Q[4] * v + Q[5]) / w });
    };
    for (let i = 1; i < 4; i++) mesh += seg(at(i / 4, 0), at(i / 4, 1)) + seg(at(0, i / 4), at(1, i / 4));
  }
  $svg("meshPath").setAttribute("d", mesh);
  $svg("meshUnder").setAttribute("d", mesh);

  let grid = "";
  if (S.grid) {
    for (const t of [1 / 3, 2 / 3]) {
      grid += seg({ x: fx + fw * t, y: fy }, { x: fx + fw * t, y: fy + fh });
      grid += seg({ x: fx, y: fy + fh * t }, { x: fx + fw, y: fy + fh * t });
    }
  }
  $svg("gridPath").setAttribute("d", grid);

  const showGhosts = !placing && !compare;
  const ghostPts = S.src.map(toStage);
  $svg("ghostPath").setAttribute("d", showGhosts ? ghostPts.map((g, i) => seg(g, stagePts[i])).join("") : "");
  ghostEls.forEach((el, i) => {
    el.hidden = !showGhosts;
    place(el, ghostPts[i].x, ghostPts[i].y);
  });

  // Points and the drag-inside area
  const quadGrab = $("quadGrab");
  quadGrab.hidden = compare;
  quadGrab.style.clipPath = "polygon(" + stagePts.map((p) => r2(p.x) + "px " + r2(p.y) + "px").join(",") + ")";
  pointEls.forEach((el, i) => {
    el.hidden = compare;
    place(el, stagePts[i].x, stagePts[i].y);
    el.classList.toggle("is-selected", S.selected === i);
    el.classList.toggle("is-dragging", S.dragging && S.selected === i);
    el.setAttribute("aria-label", "Point " + (i + 1) + ", x " + Math.round(pts[i].x) + ", y " + Math.round(pts[i].y) + " pixels");
  });

  // Compare
  const comparePhoto = $("comparePhoto");
  const divider = $("divider");
  const tagShifted = $("tagShifted");
  const tagOriginal = $("tagOriginal");
  comparePhoto.hidden = !compare;
  divider.hidden = !compare;
  tagShifted.hidden = !compare;
  tagOriginal.hidden = !compare;
  if (compare) {
    place(comparePhoto, fx, fy, fw, fh);
    comparePhoto.style.clipPath = "inset(0 0 0 " + r2(fw * S.split) + "px)";
    place(divider, fx + fw * S.split, fy, null, fh);
    place(tagShifted, fx + 8, fy + 8);
    tagOriginal.style.top = r2(fy + 8) + "px";
    tagOriginal.style.right = r2(sw - fx - fw + 8) + "px";
  }

  // Loupe
  const showLoupe = placing && S.dragging;
  const loupe = $("loupe");
  loupe.hidden = !showLoupe;
  if (showLoupe) {
    const a = stagePts[S.selected];
    const p = pts[S.selected];
    const z = Math.max(2, s * 4);
    let lx = a.x + 30;
    let ly = a.y - 202;
    if (lx + 144 > sw - 8) lx = a.x - 174;
    if (ly < 8) ly = a.y + 30;
    place(loupe, lx, ly);
    const img = $("loupeImg");
    img.style.width = W + "px";
    img.style.height = H + "px";
    img.style.transform = "translate(" + r2(72 - z * p.x) + "px," + r2(72 - z * p.y) + "px) scale(" + z + ")";
    $("loupeLabel").textContent = Math.round(p.x) + ", " + Math.round(p.y) + " px";
  }
}
