import { describe, expect, it } from "vitest";
import { homography, insetPoints } from "../engine/index.ts";
import { gapPercent, outputSize } from "./output-size.ts";

const W = 1600;
const H = 1067;
const q = insetPoints(W, H);
const none = homography(q, q);
const slid = homography(q, q.map((p) => ({ x: p.x + W / 4, y: p.y })));

describe("outputSize", () => {
  it("is the photo's size with no shift", () => {
    expect(outputSize(none, W, H, "original", "crop")).toEqual({ width: 1600, height: 1067 });
    expect(outputSize(none, W, H, "original", "stretch")).toEqual({ width: 1600, height: 1067 });
  });

  it("scales to the chosen long edge and keeps the aspect ratio", () => {
    expect(outputSize(none, W, H, "1280", "stretch")).toEqual({ width: 1280, height: 854 });
    expect(outputSize(none, W, H, "3840", "clear")).toEqual({ width: 3840, height: 2561 });
    expect(outputSize(none, 1000, 2000, "1280", "clear")).toEqual({ width: 640, height: 1280 });
  });

  it("shrinks to the crop at original size, and reaches the long edge otherwise", () => {
    expect(outputSize(slid, W, H, "original", "crop")).toEqual({ width: 1200, height: 800 });
    expect(outputSize(slid, W, H, "1280", "crop")).toEqual({ width: 1280, height: 854 });
  });

  it("ignores the crop for the other options", () => {
    expect(outputSize(slid, W, H, "original", "stretch")).toEqual({ width: 1600, height: 1067 });
  });

  it("is null when nothing is left to crop to", () => {
    const away = homography(q, q.map((p) => ({ x: p.x + W * 3, y: p.y })));
    expect(outputSize(away, W, H, "original", "crop")).toBeNull();
    expect(outputSize(away, W, H, "original", "stretch")).not.toBeNull();
  });
});

describe("gapPercent", () => {
  it("reports the empty share of the frame", () => {
    expect(gapPercent(none, W, H)).toBeCloseTo(0, 9);
    expect(gapPercent(slid, W, H)).toBeCloseTo(25, 9);
  });
});
