import { corners, coverage, cropRect, project, type Matrix } from "../engine/index.ts";
import type { EmptyAreas } from "./state.ts";

/** What the export will produce. Mirrors `plan` in src-tauri/src/export.rs. Null if nothing is left to crop to. */
export function outputSize(
  M: Matrix,
  W: number,
  H: number,
  size: string,
  empty: EmptyAreas,
): { width: number; height: number } | null {
  const view = empty === "crop" ? cropRect(M, W, H) : { x: 0, y: 0, w: W, h: H };
  if (!view) return null;
  const k = size === "original" ? 1 : Number(size) / Math.max(view.w, view.h);
  const width = Math.round(view.w * k);
  const height = Math.round(view.h * k);
  if (width < 1 || height < 1) return null;
  return { width, height };
}

/** The empty share of the frame, as a percentage. */
export function gapPercent(M: Matrix, W: number, H: number): number {
  return Math.max(0, (1 - coverage(corners(W, H).map((c) => project(M, c)), W, H)) * 100);
}

export const FRAME_FULL_BELOW = 0.05;

export const EXTENSIONS = { jpeg: ".jpg", png: ".png", tiff: ".tif", webp: ".webp" } as const;
