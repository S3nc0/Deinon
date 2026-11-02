import {
  applyCam,
  estimateCam,
  flatCam,
  homography,
  insetPoints,
  isValid,
  project,
  type Camera,
  type Matrix,
  type Point,
  type Quad,
} from "../engine/index.ts";
import * as history from "./history.ts";
import { S, set, type EditorState } from "./state.ts";

export const activePts = () => (S.tool === "place" ? S.src : S.dst);

export function openPhoto(url: string, name: string, W: number, H: number, cam: Camera) {
  cancelTween();
  const src = insetPoints(W, H);
  const flat = flatCam();
  const dst = applyCam(src, flat, cam, W, H);
  set({
    view: "editor",
    photo: url,
    fileName: name,
    W,
    H,
    src,
    dst,
    goodSrc: src,
    good: dst,
    base: src,
    camBase: flat,
    cam,
    invalid: false,
    zoom: 1,
    compare: false,
    tool: "shift",
    past: [],
    future: [],
    exported: null,
    exportError: "",
    savedMatrix: homography(src, dst),
    exportName: name.replace(/\.[^.]+$/, "") + "_shifted",
  });
}

export const currentMatrix = (): Matrix => homography(S.goodSrc, S.good);

/** The current warp differs from what was opened or last exported. */
export function isDirty(): boolean {
  if (S.view !== "editor" || !S.savedMatrix) return false;
  const m = currentMatrix();
  const saved = S.savedMatrix;
  return m.some((v, i) => Math.abs(v - saved[i]) > 1e-9);
}

export function markSaved() {
  set({ savedMatrix: currentMatrix() });
}

/* ---------- Edits ---------- */

export function shiftTo(dst: Quad) {
  if (isValid(S.src, dst, S.W, S.H)) {
    const cam = estimateCam(homography(S.src, dst), S.W, S.H);
    set({ dst, goodSrc: S.src, good: dst, base: dst, camBase: cam, cam, invalid: false });
  } else {
    set({ dst, invalid: true });
  }
}

/** Moving an anchor keeps the warp: the new anchor lands where the current warp already puts it. */
export function placeTo(src: Quad, M: Matrix) {
  const dst = src.map((p) => {
    const q = project(M, p);
    return { x: q.x, y: q.y };
  });
  if (isValid(src, dst, S.W, S.H)) set({ src, dst, goodSrc: src, good: dst, base: dst, camBase: S.cam, invalid: false });
  else set({ src, invalid: true });
}

/** If the points are crossed when you let go, they snap back to the last good position. */
export function settle() {
  if (S.invalid) set({ src: S.goodSrc, dst: S.good, invalid: false });
}

export function setPoint(i: number, p: Point) {
  finishTween();
  if (S.tool === "place") {
    const src = S.src.slice();
    src[i] = p;
    placeTo(src, homography(S.goodSrc, S.good));
  } else {
    const dst = S.dst.slice();
    dst[i] = p;
    shiftTo(dst);
  }
}

export function setCam(key: keyof Camera, value: number) {
  finishTween();
  history.remember("cam-" + key);
  const cam = { ...S.cam, [key]: value };
  const dst = applyCam(S.base, S.camBase, cam, S.W, S.H);
  if (isValid(S.src, dst, S.W, S.H)) set({ cam, dst, goodSrc: S.src, good: dst, invalid: false });
  else set({ cam, dst, invalid: true });
}

/* ---------- Presets and reset ---------- */

let tween: number | null = null;
let tweenEnd: Partial<EditorState> | null = null;

export function cancelTween() {
  if (tween !== null) cancelAnimationFrame(tween);
  tween = null;
  tweenEnd = null;
}

/** Jumps a running animation to its end, so a new edit builds on the finished pose. */
function finishTween() {
  if (tweenEnd) {
    const end = tweenEnd;
    cancelTween();
    set(end);
  }
}

const shouldAnimate = () =>
  typeof requestAnimationFrame === "function" && !matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Animates to a camera pose over 240 ms, measured from the unshifted photo. */
export function poseTo(cam: Camera) {
  const flat = flatCam();
  const dst = applyCam(S.src, flat, cam, S.W, S.H);
  if (!isValid(S.src, dst, S.W, S.H)) return;
  history.remember(null);
  cancelTween();
  const done: Partial<EditorState> = { dst, goodSrc: S.src, good: dst, base: S.src, camBase: flat, cam, invalid: false };
  if (!shouldAnimate()) {
    set(done);
    return;
  }
  tweenEnd = done;
  const from = S.good;
  const fromCam = S.cam;
  const t0 = performance.now();
  const step = (now: number) => {
    const t = Math.min(1, (now - t0) / 240);
    if (t >= 1) {
      tween = null;
      tweenEnd = null;
      set(done);
      return;
    }
    const e = 1 - Math.pow(1 - t, 3);
    const mid = from.map((p, i) => ({ x: p.x + (dst[i].x - p.x) * e, y: p.y + (dst[i].y - p.y) * e }));
    const midCam: Camera = {
      yaw: fromCam.yaw + (cam.yaw - fromCam.yaw) * e,
      pitch: fromCam.pitch + (cam.pitch - fromCam.pitch) * e,
      roll: fromCam.roll + (cam.roll - fromCam.roll) * e,
      fov: fromCam.fov + (cam.fov - fromCam.fov) * e,
    };
    set({ dst: mid, good: mid, cam: midCam, invalid: false });
    tween = requestAnimationFrame(step);
  };
  tween = requestAnimationFrame(step);
}

export function undo() {
  cancelTween();
  history.undo();
}

export function redo() {
  cancelTween();
  history.redo();
}
