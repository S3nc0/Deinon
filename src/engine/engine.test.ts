import { describe, expect, it } from "vitest";
import fixtures from "../../fixtures/homography.json";
import {
  applyCam,
  convexSign,
  corners,
  coverage,
  cropRect,
  estimateCam,
  flatCam,
  homography,
  insetPoints,
  isValid,
  project,
  toCssMatrix,
  type Quad,
} from "./index.ts";

const W = fixtures.frame.width;
const H = fixtures.frame.height;
const inset = insetPoints(W, H);
const warped = (m: number[]) => corners(W, H).map((c) => project(m, c));

describe("homography", () => {
  it("maps each source point onto its target", () => {
    const src: Quad = [{ x: 10, y: 20 }, { x: 900, y: 40 }, { x: 870, y: 700 }, { x: 30, y: 650 }];
    const dst: Quad = [{ x: -40, y: 60 }, { x: 1000, y: 10 }, { x: 810, y: 760 }, { x: 90, y: 600 }];
    const m = homography(src, dst);
    src.forEach((p, i) => {
      const q = project(m, p);
      expect(Math.abs(q.x - dst[i].x)).toBeLessThan(1e-6);
      expect(Math.abs(q.y - dst[i].y)).toBeLessThan(1e-6);
    });
  });

  it("writes the CSS matrix in column order", () => {
    const m = [1, 2, 3, 4, 5, 6, 7, 8, 1];
    expect(toCssMatrix(m)).toBe("matrix3d(1,4,0,7,2,5,0,8,0,0,1,0,3,6,0,1)");
  });
});

describe("fixtures from the reference", () => {
  it("has at least 6 cases", () => {
    expect(fixtures.cases.length).toBeGreaterThanOrEqual(6);
  });

  for (const c of fixtures.cases) {
    describe(c.name, () => {
      const m = homography(c.src, c.dst);

      it("gives the same matrix", () => {
        m.forEach((v, i) => expect(Math.abs(v - c.matrix[i])).toBeLessThan(1e-6));
      });

      it("maps the sample points to the same place", () => {
        for (const s of c.samples) {
          const q = project(m, s.from);
          expect(Math.abs(q.x - s.to.x)).toBeLessThan(1e-6);
          expect(Math.abs(q.y - s.to.y)).toBeLessThan(1e-6);
        }
      });

      it("gives the same coverage", () => {
        expect(Math.abs(coverage(warped(m), W, H) - c.coverage)).toBeLessThan(1e-9);
      });
    });
  }
});

describe("estimateCam", () => {
  const single: [string, Partial<ReturnType<typeof flatCam>>][] = [
    ["yaw", { yaw: 27 }],
    ["yaw", { yaw: -35 }],
    ["pitch", { pitch: 18 }],
    ["pitch", { pitch: -30 }],
    ["roll", { roll: 22 }],
    ["roll", { roll: -40 }],
    ["fov", { fov: 60 }],
    ["fov", { fov: 110 }],
  ];

  for (const [axis, patch] of single) {
    it(`recovers ${axis} ${Object.values(patch)[0]}° on its own`, () => {
      const cam = { ...flatCam(), ...patch };
      const dst = applyCam(inset, flatCam(), cam, W, H);
      const got = estimateCam(homography(inset, dst), W, H);
      const key = axis as "yaw" | "pitch" | "roll" | "fov";
      expect(Math.abs(got[key] - cam[key])).toBeLessThan(0.01);
    });
  }
});

describe("isValid", () => {
  it("accepts no shift and a moderate shift", () => {
    expect(isValid(inset, inset, W, H)).toBe(true);
    const dst = applyCam(inset, flatCam(), { ...flatCam(), yaw: 20, pitch: 10 }, W, H);
    expect(isValid(inset, dst, W, H)).toBe(true);
  });

  it("rejects crossed points", () => {
    const crossed = [inset[0], inset[2], inset[1], inset[3]];
    expect(convexSign(crossed)).toBe(0);
    expect(isValid(inset, crossed, W, H)).toBe(false);
  });

  it("rejects a shape dented inwards", () => {
    const dented = [inset[0], inset[1], { x: W / 2, y: H / 2 - 100 }, inset[3]];
    expect(isValid(inset, dented, W, H)).toBe(false);
  });

  it("rejects a flipped shape", () => {
    const flipped = [inset[1], inset[0], inset[3], inset[2]];
    expect(convexSign(flipped)).toBe(-convexSign(inset));
    expect(isValid(inset, flipped, W, H)).toBe(false);
  });

  it("rejects a warp that passes behind the camera", () => {
    // The right side converges so hard that the photo's far corners end up behind the horizon.
    const dst: Quad = [
      { x: 320, y: 213 },
      { x: 400, y: 500 },
      { x: 400, y: 560 },
      { x: 320, y: 853 },
    ];
    expect(convexSign(dst)).toBe(convexSign(inset));
    const m = homography(inset, dst);
    expect(corners(W, H).some((c) => project(m, c).w <= 0.05)).toBe(true);
    expect(isValid(inset, dst, W, H)).toBe(false);
  });
});

describe("coverage", () => {
  it("is 1 with no shift", () => {
    expect(coverage(warped(homography(inset, inset)), W, H)).toBeCloseTo(1, 12);
  });

  it("matches a hand-worked case with gaps", () => {
    // The photo slides right by a quarter of the frame, leaving a quarter empty.
    const dst = inset.map((p) => ({ x: p.x + W / 4, y: p.y }));
    expect(coverage(warped(homography(inset, dst)), W, H)).toBeCloseTo(0.75, 9);
  });

  it("matches a hand-worked case that shrinks", () => {
    // Half the width and half the height, so a quarter of the area.
    const q: Quad = [{ x: 0, y: 0 }, { x: W / 2, y: 0 }, { x: W / 2, y: H / 2 }, { x: 0, y: H / 2 }];
    expect(coverage(q, W, H)).toBeCloseTo(0.25, 12);
  });

  it("is 0 when the photo has left the frame", () => {
    const q = corners(W, H).map((p) => ({ x: p.x + W * 2, y: p.y }));
    expect(coverage(q, W, H)).toBe(0);
  });
});

describe("cropRect", () => {
  const inside = (r: { x: number; y: number; w: number; h: number }, m: number[]) => {
    // Every corner of the crop must lie inside the warped photo, so its anchor in the photo is in bounds.
    const poly = warped(m);
    const sign = Math.sign(
      poly.reduce((sum, a, i) => {
        const b = poly[(i + 1) % 4];
        return sum + a.x * b.y - b.x * a.y;
      }, 0),
    );
    const pts = [
      { x: r.x, y: r.y },
      { x: r.x + r.w, y: r.y },
      { x: r.x + r.w, y: r.y + r.h },
      { x: r.x, y: r.y + r.h },
    ];
    return pts.every((p) =>
      poly.every((a, i) => {
        const b = poly[(i + 1) % 4];
        return sign * ((b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)) >= -1e-6;
      }),
    );
  };

  it("is the whole frame with no shift", () => {
    const r = cropRect(homography(inset, inset), W, H)!;
    expect(r.x).toBeCloseTo(0, 6);
    expect(r.y).toBeCloseTo(0, 6);
    expect(r.w).toBeCloseTo(W, 6);
    expect(r.h).toBeCloseTo(H, 6);
  });

  it("matches a hand-worked slide", () => {
    // Sliding right by a quarter leaves a 3/4 wide photo, so the best crop is 3/4 scale.
    const dst = inset.map((p) => ({ x: p.x + W / 4, y: p.y }));
    const r = cropRect(homography(inset, dst), W, H)!;
    expect(r.w).toBeCloseTo(W * 0.75, 6);
    expect(r.h).toBeCloseTo(H * 0.75, 6);
    expect(r.x).toBeCloseTo(W / 4, 6);
  });

  it("returns null when the photo has left the frame", () => {
    const dst = inset.map((p) => ({ x: p.x + W * 3, y: p.y }));
    expect(cropRect(homography(inset, dst), W, H)).toBeNull();
  });

  for (const c of fixtures.cases) {
    it(`fits inside the photo, keeps the aspect ratio and cannot grow: ${c.name}`, () => {
      const m = homography(c.src, c.dst);
      const r = cropRect(m, W, H)!;
      expect(r).not.toBeNull();
      expect(r.w / r.h).toBeCloseTo(W / H, 9);
      expect(r.x).toBeGreaterThanOrEqual(-1e-6);
      expect(r.y).toBeGreaterThanOrEqual(-1e-6);
      expect(r.x + r.w).toBeLessThanOrEqual(W + 1e-6);
      expect(r.y + r.h).toBeLessThanOrEqual(H + 1e-6);
      expect(inside(r, m)).toBe(true);

      // A grid search around it finds nothing larger that still fits.
      const grow = { w: r.w * 1.005, h: r.h * 1.005 };
      for (let i = -40; i <= 40; i++) {
        for (let j = -40; j <= 40; j++) {
          const t = { x: r.x + (i * W) / 400, y: r.y + (j * H) / 400, ...grow };
          const fits = t.x >= 0 && t.y >= 0 && t.x + t.w <= W && t.y + t.h <= H && inside(t, m);
          expect(fits).toBe(false);
        }
      }
    });

    it(`matches the fixture: ${c.name}`, () => {
      const r = cropRect(homography(c.src, c.dst), W, H)!;
      expect(Math.abs(r.x - c.crop!.x)).toBeLessThan(1e-6);
      expect(Math.abs(r.y - c.crop!.y)).toBeLessThan(1e-6);
      expect(Math.abs(r.w - c.crop!.w)).toBeLessThan(1e-6);
      expect(Math.abs(r.h - c.crop!.h)).toBeLessThan(1e-6);
    });
  }
});

describe("insetPoints", () => {
  it("starts 20% in from the edges", () => {
    expect(insetPoints(1000, 500)).toEqual([
      { x: 200, y: 100 },
      { x: 800, y: 100 },
      { x: 800, y: 400 },
      { x: 200, y: 400 },
    ]);
  });
});
