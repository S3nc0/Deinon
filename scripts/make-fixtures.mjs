// Writes fixtures/homography.json: the test data shared by the TypeScript and Rust tests.
//
// The matrices come from the maths in design/index.html itself, run as written, so the tests
// check both ports against the reference and not against themselves. The crop rectangles have
// no reference version, so they come from src/engine/crop.ts and check that Rust agrees with it.
//
// Run with: npm run fixtures   (needs a Node version that runs .ts files directly)
import { readFileSync, writeFileSync } from "node:fs";
import { cropRect } from "../src/engine/crop.ts";

const root = new URL("..", import.meta.url);
const html = readFileSync(new URL("design/index.html", root), "utf8");
const start = html.indexOf("const clamp =");
const end = html.indexOf("/* ---------- State ---------- */");
if (start < 0 || end < 0) throw new Error("Could not find the maths in design/index.html");

const ref = new Function(
  html.slice(start, end) +
    "\nreturn { homography, project, applyCam, flatCam, insetPoints, isValid, corners, coverage };",
)();

const W = 1600;
const H = 1067;
const flat = ref.flatCam();
const inset = ref.insetPoints(W, H);
const cam = (patch) => ({ ...flat, ...patch });

const cases = [
  { name: "no shift", src: inset, dst: inset },
  { name: "pure yaw", src: inset, dst: ref.applyCam(inset, flat, cam({ yaw: 20 }), W, H) },
  { name: "pure pitch", src: inset, dst: ref.applyCam(inset, flat, cam({ pitch: -16 }), W, H) },
  { name: "pure roll", src: inset, dst: ref.applyCam(inset, flat, cam({ roll: 10 }), W, H) },
  { name: "zoom in", src: inset, dst: ref.applyCam(inset, flat, cam({ fov: 60 }), W, H) },
  {
    name: "strong mixed shift",
    src: inset,
    dst: ref.applyCam(inset, flat, cam({ yaw: 28, pitch: 19, roll: -14, fov: 96 }), W, H),
  },
  {
    name: "moved anchors",
    src: [
      { x: 212.5, y: 148.25 },
      { x: 1391, y: 203.75 },
      { x: 1288.5, y: 941 },
      { x: 301, y: 877.5 },
    ],
    dst: [
      { x: 180, y: 96 },
      { x: 1446.25, y: 240.5 },
      { x: 1210, y: 1003 },
      { x: 348.75, y: 812 },
    ],
  },
];

const samples = [
  { x: 0, y: 0 },
  { x: W, y: 0 },
  { x: W, y: H },
  { x: 0, y: H },
  { x: W / 2, y: H / 2 },
  { x: 137.5, y: 941.25 },
];

const round = (n) => Number(n.toFixed(12));
const roundPoint = (p) => ({ x: round(p.x), y: round(p.y) });

const out = {
  frame: { width: W, height: H },
  cases: cases.map((c) => {
    if (!ref.isValid(c.src, c.dst, W, H)) throw new Error(`Invalid fixture: ${c.name}`);
    const m = ref.homography(c.src, c.dst);
    const crop = cropRect(m, W, H);
    return {
      name: c.name,
      src: c.src.map(roundPoint),
      dst: c.dst.map(roundPoint),
      matrix: m.map((v) => Number(v.toPrecision(15))),
      samples: samples.map((p) => {
        const q = ref.project(m, p);
        return { from: p, to: roundPoint(q) };
      }),
      coverage: round(ref.coverage(ref.corners(W, H).map((p) => ref.project(m, p)), W, H)),
      crop: crop && { x: round(crop.x), y: round(crop.y), w: round(crop.w), h: round(crop.h) },
    };
  }),
};

writeFileSync(new URL("fixtures/homography.json", root), JSON.stringify(out, null, 2) + "\n");
console.log(`Wrote ${out.cases.length} cases to fixtures/homography.json`);
