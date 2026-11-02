import { BASE_FOV, flatCam, type Camera } from "../engine/index.ts";
import * as api from "../native/api.ts";
import { isDirty, openPhoto } from "./edits.ts";
import { clearMessage, showLoading, showMessage } from "./messages.ts";
import { S } from "./state.ts";

const SUPPORTED = /\.(jpe?g|png|tiff?|webp)$/i;
export const UNSUPPORTED_DROP = "Those files aren't supported images. Drop a JPEG, PNG, TIFF or WebP.";

/** The first dropped file the app can open. */
export const firstSupported = (paths: string[]) => paths.find((p) => SUPPORTED.test(p)) ?? null;

const baseName = (path: string) => path.split(/[\\/]/).pop() ?? path;

let busy = false;

async function okToReplace(): Promise<boolean> {
  return !isDirty() || api.confirmDiscard();
}

/** Decodes the preview before the editor shows it, so the stage never appears empty. */
async function show(opened: api.OpenedPhoto, cam: Camera) {
  const url = api.previewUrl(opened.preview);
  const image = new Image();
  image.src = url;
  await image.decode();
  const old = S.photo;
  openPhoto(url, opened.fileName, opened.width, opened.height, cam);
  // The old preview is still on screen until the next frame.
  if (old) setTimeout(() => URL.revokeObjectURL(old), 2000);
}

async function load(label: string, open: () => Promise<api.OpenedPhoto>, cam: Camera) {
  if (busy) return;
  busy = true;
  clearMessage();
  showLoading("Opening " + label);
  try {
    await show(await open(), cam);
  } catch (e) {
    showMessage(api.errorMessage(e));
  } finally {
    busy = false;
    showLoading(null);
  }
}

export async function requestOpen() {
  if (busy) return;
  try {
    const path = await api.choosePhoto();
    if (!path || !(await okToReplace())) return;
    await load(baseName(path), () => api.openPhoto(path), flatCam());
  } catch (e) {
    showMessage(api.errorMessage(e));
  }
}

export async function requestSample() {
  if (busy || !(await okToReplace())) return;
  await load("the sample photo", api.openSample, { yaw: 12, pitch: -6, roll: 2, fov: BASE_FOV });
}

export async function openDropped(paths: string[]) {
  if (busy) return;
  const path = firstSupported(paths);
  if (!path) {
    showMessage(UNSUPPORTED_DROP);
    return;
  }
  if (!(await okToReplace())) return;
  await load(baseName(path), () => api.openPhoto(path), flatCam());
}
