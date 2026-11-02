import { clipToFrame } from "./coverage.ts";
import { corners, project } from "./homography.ts";
import type { Matrix, Point, Rect } from "./types.ts";

/**
 * The largest rectangle with the frame's aspect ratio that fits fully inside the warped photo
 * and the frame. Returns null if no usable rectangle exists.
 *
 * The clipped photo is convex, so a rectangle at (x, y) scaled by t fits exactly when
 * a·x + b·y + d·t + c ≥ 0 for every edge. Maximising t over those constraints is a small
 * linear program, and its best answer is where 3 constraints meet. This matches
 * `crop_rect` in src-tauri/src/warp.rs, and both are tested against fixtures/homography.json.
 */
export function cropRect(M: Matrix, W: number, H: number): Rect | null {
  const poly = clipToFrame(corners(W, H).map((c) => toPoint(project(M, c))), W, H);
  const n = poly.length;
  if (n < 3) return null;

  let area = 0;
  poly.forEach((p, i) => {
    const q = poly[(i + 1) % n];
    area += p.x * q.y - q.x * p.y;
  });
  if (Math.abs(area) < 1e-9) return null;
  const sign = area > 0 ? 1 : -1;

  // Normalised, so the feasibility tolerance below is a distance in pixels.
  const cons: [number, number, number, number][] = [];
  for (let i = 0; i < n; i++) {
    const p = poly[i];
    const q = poly[(i + 1) % n];
    const len = Math.hypot(q.x - p.x, q.y - p.y);
    if (len < 1e-9) continue;
    const a = (-(q.y - p.y) * sign) / len;
    const b = ((q.x - p.x) * sign) / len;
    const c = -(a * p.x + b * p.y);
    cons.push([a, b, Math.min(0, a * W) + Math.min(0, b * H), c]);
  }

  const found: { x: number; y: number; t: number }[] = [];
  let tMax = 0;
  for (let i = 0; i < cons.length; i++) {
    for (let j = i + 1; j < cons.length; j++) {
      for (let k = j + 1; k < cons.length; k++) {
        const s = solve3(cons[i], cons[j], cons[k]);
        if (!s || s.t <= 0) continue;
        if (!cons.every(([a, b, d, c]) => a * s.x + b * s.y + d * s.t + c >= -1e-7)) continue;
        found.push(s);
        tMax = Math.max(tMax, s.t);
      }
    }
  }

  // The best size can have many positions, such as when only the top and bottom edges limit it.
  // They form a line, so take its middle: the same answer every time, and well placed.
  const best = found.filter((s) => s.t >= tMax - 1e-7);
  if (!best.length) return null;
  const t = Math.min(...best.map((s) => s.t));
  const xs = best.map((s) => s.x);
  const ys = best.map((s) => s.y);
  return {
    x: (Math.min(...xs) + Math.max(...xs)) / 2,
    y: (Math.min(...ys) + Math.max(...ys)) / 2,
    w: t * W,
    h: t * H,
  };
}

function toPoint(p: Point): Point {
  return { x: p.x, y: p.y };
}

/** Where 3 constraints meet, by Cramer's rule. */
function solve3(
  p: [number, number, number, number],
  q: [number, number, number, number],
  r: [number, number, number, number],
): { x: number; y: number; t: number } | null {
  const det3 = (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) =>
    a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
  const det = det3(p[0], p[1], p[2], q[0], q[1], q[2], r[0], r[1], r[2]);
  if (Math.abs(det) < 1e-12) return null;
  return {
    x: det3(-p[3], p[1], p[2], -q[3], q[1], q[2], -r[3], r[1], r[2]) / det,
    y: det3(p[0], -p[3], p[2], q[0], -q[3], q[2], r[0], -r[3], r[2]) / det,
    t: det3(p[0], p[1], -p[3], q[0], q[1], -q[3], r[0], r[1], -r[3]) / det,
  };
}
