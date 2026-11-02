import { corners, project } from "./homography.ts";
import type { Camera, Matrix, Quad } from "./types.ts";

export const BASE_FOV = 84;

const rad = (d: number) => (d * Math.PI) / 180;
const deg = (r: number) => (r * 180) / Math.PI;
const clamp = (v: number, a: number, b: number) => Math.min(b, Math.max(a, v));

export const flatCam = (): Camera => ({ yaw: 0, pitch: 0, roll: 0, fov: BASE_FOV });

const focal = (W: number) => W / 2 / Math.tan(rad(BASE_FOV / 2));

/** Turns the virtual camera from one pose to another. The photo is a plane rotating about its centre. */
export function applyCam(pts: Quad, from: Camera, to: Camera, W: number, H: number): Quad {
  const f = focal(W);
  const yaw = rad(to.yaw - from.yaw);
  const pitch = rad(to.pitch - from.pitch);
  const roll = rad(to.roll - from.roll);
  const zoom = Math.tan(rad(from.fov / 2)) / Math.tan(rad(to.fov / 2));
  const cx = W / 2;
  const cy = H / 2;
  return pts.map((p) => {
    const x = p.x - cx;
    const y = p.y - cy;
    const x1 = x * Math.cos(roll) - y * Math.sin(roll);
    const y1 = x * Math.sin(roll) + y * Math.cos(roll);
    const y2 = y1 * Math.cos(pitch);
    const z2 = -y1 * Math.sin(pitch);
    const x3 = x1 * Math.cos(yaw) - z2 * Math.sin(yaw);
    const z3 = x1 * Math.sin(yaw) + z2 * Math.cos(yaw);
    const k = (f / (f + z3)) * zoom;
    return { x: cx + x3 * k, y: cy + y2 * k };
  });
}

/** Reads the camera angles back out of a hand-dragged shape, so the sliders stay in step. */
export function estimateCam(M: Matrix, W: number, H: number): Camera {
  const f = focal(W);
  const c = corners(W, H).map((p) => project(M, p));
  const len = (a: { x: number; y: number }, b: { x: number; y: number }) => Math.hypot(a.x - b.x, a.y - b.y);
  const hL = len(c[3], c[0]);
  const hR = len(c[2], c[1]);
  const wT = len(c[1], c[0]);
  const wB = len(c[2], c[3]);
  const yaw = deg(Math.asin(clamp(((hL - hR) / (hL + hR)) * ((2 * f) / W), -1, 1)));
  const pitch = deg(Math.asin(clamp(((wB - wT) / (wB + wT)) * ((2 * f) / H), -1, 1)));
  const roll = deg((Math.atan2(c[1].y - c[0].y, c[1].x - c[0].x) + Math.atan2(c[2].y - c[3].y, c[2].x - c[3].x)) / 2);
  let area = 0;
  c.forEach((a, i) => {
    const b = c[(i + 1) % 4];
    area += a.x * b.y - b.x * a.y;
  });
  const fov = 2 * deg(Math.atan(Math.tan(rad(BASE_FOV / 2)) / Math.sqrt(Math.abs(area) / 2 / (W * H))));
  return { yaw, pitch, roll, fov: clamp(fov, 40, 120) };
}

/** Where the points start: 20% in from the edges. */
export function insetPoints(W: number, H: number): Quad {
  const x0 = W * 0.2;
  const x1 = W * 0.8;
  const y0 = H * 0.2;
  const y1 = H * 0.8;
  return [
    { x: x0, y: y0 },
    { x: x1, y: y0 },
    { x: x1, y: y1 },
    { x: x0, y: y1 },
  ];
}
