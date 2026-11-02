import type { Point, Quad } from "./types.ts";

type Inside = (p: Point) => boolean;
type Cross = (a: Point, b: Point) => Point;

function clipEdge(pts: Quad, inside: Inside, cross: Cross): Quad {
  const out: Quad = [];
  pts.forEach((a, i) => {
    const b = pts[(i + 1) % pts.length];
    if (inside(a)) out.push(a);
    if (inside(a) !== inside(b)) out.push(cross(a, b));
  });
  return out;
}

/** Clips a shape to the frame (Sutherland-Hodgman). */
export function clipToFrame(poly: Quad, W: number, H: number): Quad {
  const atX = (X: number): Cross => (a, b) => ({ x: X, y: a.y + ((X - a.x) / (b.x - a.x)) * (b.y - a.y) });
  const atY = (Y: number): Cross => (a, b) => ({ x: a.x + ((Y - a.y) / (b.y - a.y)) * (b.x - a.x), y: Y });
  let p = clipEdge(poly, (a) => a.x >= 0, atX(0));
  p = clipEdge(p, (a) => a.x <= W, atX(W));
  p = clipEdge(p, (a) => a.y >= 0, atY(0));
  return clipEdge(p, (a) => a.y <= H, atY(H));
}

/** Share of the output frame that the warped photo covers, from 0 to 1. */
export function coverage(poly: Quad, W: number, H: number): number {
  const p = clipToFrame(poly, W, H);
  let area = 0;
  p.forEach((a, i) => {
    const b = p[(i + 1) % p.length];
    area += a.x * b.y - b.x * a.y;
  });
  return Math.abs(area) / 2 / (W * H);
}
