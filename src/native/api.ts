// The only file, with menu.ts, that calls Tauri. The rest of the frontend goes through these.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Point } from "../engine/types.ts";

export interface OpenedPhoto {
  width: number;
  height: number;
  fileName: string;
  /** A base64 JPEG. */
  preview: string;
}

export interface ExportOptions {
  format: "jpeg" | "png" | "tiff" | "webp";
  longEdge: number | null;
  quality: number;
  empty: "crop" | "stretch" | "clear";
}

export interface ExportRequest {
  src: Point[];
  dst: Point[];
  path: string;
  options: ExportOptions;
}

export interface ExportResult {
  fileName: string;
  width: number;
  height: number;
}

export interface UiState {
  title: string;
  dirty: boolean;
  photoOpen: boolean;
  canUndo: boolean;
  canRedo: boolean;
  canCompare: boolean;
  compare: boolean;
  mesh: boolean;
  grid: boolean;
}

export interface FileDrop {
  kind: "enter" | "over" | "leave" | "drop";
  paths: string[];
}

export const choosePhoto = () => invoke<string | null>("choose_photo");
export const openPhoto = (path: string) => invoke<OpenedPhoto>("open_photo", { path });
export const openSample = () => invoke<OpenedPhoto>("open_sample");
export const chooseExportPath = (fileName: string, format: ExportOptions["format"]) =>
  invoke<string | null>("choose_export_path", { fileName, format });
export const exportPhoto = (request: ExportRequest) => invoke<ExportResult>("export_photo", { request });
export const cancelExport = () => invoke<void>("cancel_export");
export const confirmDiscard = () => invoke<boolean>("confirm_discard");
export const setUiState = (ui: UiState) => invoke<void>("set_ui_state", { ui });

export const onExportProgress = (handler: (fraction: number) => void) =>
  listen<{ fraction: number }>("export-progress", (e) => handler(e.payload.fraction));
export const onFileDrop = (handler: (drop: FileDrop) => void) =>
  listen<FileDrop>("file-drop", (e) => handler(e.payload));

/** Turns a base64 JPEG into a URL an image can show. The content security policy allows blob: images. */
export function previewUrl(base64: string): string {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" }));
}

const isAppError = (e: unknown): e is { code: string; message: string } =>
  typeof e === "object" && e !== null && "message" in e && typeof (e as { message: unknown }).message === "string";

export const isCancelled = (e: unknown) => isAppError(e) && e.code === "cancelled";

export function errorMessage(e: unknown): string {
  if (isAppError(e)) return e.message;
  if (typeof e === "string") return e;
  return "Something went wrong inside the app. Try again.";
}
