import { corners, homography, project } from "./homography.ts";
import type { Quad } from "./types.ts";

/** The winding direction of a convex shape: 1 or -1. Returns 0 if it is not convex. */
export function convexSign(q: Quad): number {
  let sign = 0;
  for (let i = 0; i < 4; i++) {
    const a = q[i];
    const b = q[(i + 1) % 4];
    const c = q[(i + 2) % 4];
    const cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
    if (Math.abs(cross) < 1) return 0;
    const s = Math.sign(cross);
    if (sign && s !== sign) return 0;
    sign = s;
  }
  return sign;
}

/** Both shapes are convex, wind the same way, and nothing passes behind the camera. */
export function isValid(src: Quad, dst: Quad, W: number, H: number): boolean {
  const a = convexSign(src);
  const b = convexSign(dst);
  if (!a || a !== b) return false;
  const M = homography(src, dst);
  return corners(W, H).every((c) => project(M, c).w > 0.05);
}
