import { flatCam, type Camera, type Matrix, type Quad } from "../engine/index.ts";

export type Tool = "place" | "shift";
export type ExportFormat = "jpeg" | "png" | "tiff" | "webp";
export type EmptyAreas = "crop" | "stretch" | "clear";

export interface Snapshot {
  src: Quad;
  dst: Quad;
  goodSrc: Quad;
  good: Quad;
  base: Quad;
  camBase: Camera;
  cam: Camera;
}

export interface ExportedFile {
  fileName: string;
  width: number;
  height: number;
}

export interface EditorState extends Snapshot {
  view: "start" | "editor";
  photo: string;
  fileName: string;
  W: number;
  H: number;
  tool: Tool;
  tab: number;
  selected: number;
  invalid: boolean;
  dragging: boolean;
  compare: boolean;
  split: number;
  mesh: boolean;
  grid: boolean;
  zoom: number;
  stageW: number;
  stageH: number;
  past: Snapshot[];
  future: Snapshot[];
  exportName: string;
  format: ExportFormat;
  size: string;
  quality: number;
  empty: EmptyAreas;
  exported: ExportedFile | null;
  exporting: boolean;
  exportProgress: number;
  exportError: string;
  /** The warp at the last export, or when the photo opened. Anything else is unexported work. */
  savedMatrix: Matrix | null;
}

export const S: EditorState = {
  view: "start",
  photo: "",
  fileName: "",
  W: 1600,
  H: 1067,
  src: [],
  dst: [],
  goodSrc: [],
  good: [],
  base: [],
  camBase: flatCam(),
  cam: flatCam(),
  tool: "shift",
  tab: 0,
  selected: 1,
  invalid: false,
  dragging: false,
  compare: false,
  split: 0.5,
  mesh: true,
  grid: false,
  zoom: 1,
  stageW: 1060,
  stageH: 760,
  past: [],
  future: [],
  exportName: "",
  format: "jpeg",
  size: "original",
  quality: 90,
  empty: "crop",
  exported: null,
  exporting: false,
  exportProgress: 0,
  exportError: "",
  savedMatrix: null,
};

/** Stage geometry from the last render, in stage pixels. Drags convert positions with it. */
export const geom = { fx: 0, fy: 0, fw: 1, fh: 1, s: 1, sw: 1, sh: 1 };

let renderer: (() => void) | null = null;
let scheduled = false;

export function setRenderer(fn: () => void) {
  renderer = fn;
}

export function set(patch: Partial<EditorState>) {
  Object.assign(S, patch);
  if (!renderer || scheduled) return;
  scheduled = true;
  requestAnimationFrame(() => {
    scheduled = false;
    renderer?.();
  });
}
