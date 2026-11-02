import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flatCam, homography } from "../engine/index.ts";
import { isDirty, markSaved, openPhoto, poseTo, setCam, setPoint, settle, placeTo, shiftTo, undo, redo } from "./edits.ts";
import { remember } from "./history.ts";
import { S, set } from "./state.ts";

const W = 1600;
const H = 1067;

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(1_000_000);
  // Let any earlier group expire, so each test starts cleanly.
  remember(null);
  openPhoto("photo.jpg", "photo.jpg", W, H, flatCam());
});

afterEach(() => {
  vi.useRealTimers();
});

const moved = (i: number, dx: number) => ({ x: S.dst[i].x + dx, y: S.dst[i].y });

describe("opening a photo", () => {
  it("starts with anchors 20% in and no shift", () => {
    expect(S.src[0]).toEqual({ x: W * 0.2, y: H * 0.2 });
    S.dst.forEach((p, i) => {
      expect(p.x).toBeCloseTo(S.src[i].x, 9);
      expect(p.y).toBeCloseTo(S.src[i].y, 9);
    });
    expect(S.invalid).toBe(false);
    expect(S.past).toEqual([]);
  });

  it("names the export after the photo", () => {
    expect(S.exportName).toBe("photo_shifted");
  });
});

describe("history", () => {
  it("groups repeated changes within 700 ms into one step", () => {
    remember("key-0");
    setPoint(0, moved(0, 1));
    vi.advanceTimersByTime(300);
    remember("key-0");
    setPoint(0, moved(0, 1));
    vi.advanceTimersByTime(300);
    remember("key-0");
    setPoint(0, moved(0, 1));
    expect(S.past.length).toBe(1);
  });

  it("starts a new step after 700 ms", () => {
    remember("key-0");
    setPoint(0, moved(0, 1));
    vi.advanceTimersByTime(701);
    remember("key-0");
    setPoint(0, moved(0, 1));
    expect(S.past.length).toBe(2);
  });

  it("starts a new step when the group changes", () => {
    remember("key-0");
    setPoint(0, moved(0, 1));
    remember("key-1");
    setPoint(1, moved(1, 1));
    expect(S.past.length).toBe(2);
  });

  it("never groups a change with no group, like a drag", () => {
    remember(null);
    remember(null);
    expect(S.past.length).toBe(2);
  });

  it("keeps at most 50 steps", () => {
    for (let i = 0; i < 60; i++) remember(null);
    expect(S.past.length).toBe(50);
  });

  it("undoes and redoes, and a new change clears redo", () => {
    const before = S.dst;
    remember(null);
    setPoint(0, moved(0, 25));
    const after = S.dst;
    expect(after).not.toBe(before);

    undo();
    expect(S.dst).toBe(before);
    expect(S.future.length).toBe(1);

    redo();
    expect(S.dst).toBe(after);
    expect(S.past.length).toBe(1);

    undo();
    remember(null);
    setPoint(0, moved(0, 5));
    expect(S.future.length).toBe(0);
  });

  it("does not group a change into a step that was just undone", () => {
    remember("key-0");
    setPoint(0, moved(0, 1));
    undo();
    remember("key-0");
    setPoint(0, moved(0, 1));
    expect(S.past.length).toBe(1);
  });

  it("does nothing with an empty history", () => {
    const dst = S.dst;
    undo();
    redo();
    expect(S.dst).toBe(dst);
  });
});

describe("shifting", () => {
  it("moves a target and updates the camera to match", () => {
    shiftTo(S.dst.map((p, i) => (i === 1 ? { x: p.x, y: p.y + 60 } : p)));
    expect(S.invalid).toBe(false);
    expect(S.good).toBe(S.dst);
    expect(S.cam).toEqual(S.camBase);
    expect(S.cam.yaw).not.toBe(0);
  });

  it("holds the last good shape when points cross, then snaps back on release", () => {
    const good = S.good;
    const crossed = [S.dst[2], S.dst[1], S.dst[0], S.dst[3]];
    shiftTo(crossed);
    expect(S.invalid).toBe(true);
    expect(S.dst).toBe(crossed);
    expect(S.good).toBe(good);

    settle();
    expect(S.invalid).toBe(false);
    expect(S.dst).toBe(good);
  });
});

describe("placing", () => {
  it("keeps the current warp when an anchor moves", () => {
    shiftTo(S.dst.map((p, i) => (i === 1 ? { x: p.x + 40, y: p.y + 60 } : p)));
    const before = homography(S.goodSrc, S.good);
    set({ tool: "place" });
    const src = S.src.map((p, i) => (i === 0 ? { x: p.x - 100, y: p.y - 50 } : p));
    placeTo(src, before);
    expect(S.invalid).toBe(false);
    expect(S.src).toBe(src);
    const after = homography(S.goodSrc, S.good);
    after.forEach((v, i) => expect(v).toBeCloseTo(before[i], 9));
    set({ tool: "shift" });
  });
});

describe("the virtual camera", () => {
  it("moves all 4 points", () => {
    const start = S.dst;
    setCam("yaw", 20);
    expect(S.cam.yaw).toBe(20);
    S.dst.forEach((p, i) => expect(p).not.toEqual(start[i]));
  });

  it("builds on a hand-dragged shape", () => {
    shiftTo(S.dst.map((p, i) => (i === 0 ? { x: p.x - 30, y: p.y - 20 } : p)));
    const dragged = S.dst;
    const yaw = S.cam.yaw;
    setCam("yaw", yaw);
    S.dst.forEach((p, i) => {
      expect(p.x).toBeCloseTo(dragged[i].x, 9);
      expect(p.y).toBeCloseTo(dragged[i].y, 9);
    });
  });

  it("stays where it is when invalid, until it is eased back", () => {
    // In a tall photo, a strong pitch swings the top and bottom points behind the camera.
    openPhoto("tall.jpg", "tall.jpg", 1000, 4000, flatCam());
    const good = S.good;
    setCam("pitch", 45);
    expect(S.invalid).toBe(true);
    expect(S.cam.pitch).toBe(45);
    expect(S.good).toBe(good);

    // Letting go does not move a slider. Easing it back does.
    expect(S.cam.pitch).toBe(45);
    setCam("pitch", 2);
    expect(S.invalid).toBe(false);
    expect(S.good).not.toBe(good);
  });

  it("groups changes to one slider into one undo step", () => {
    setCam("yaw", 5);
    vi.advanceTimersByTime(100);
    setCam("yaw", 6);
    vi.advanceTimersByTime(100);
    setCam("yaw", 7);
    expect(S.past.length).toBe(1);
    undo();
    expect(S.cam.yaw).toBe(0);
  });
});

describe("presets and reset", () => {
  it("measures a preset from the unshifted photo and goes back with reset", () => {
    shiftTo(S.dst.map((p, i) => (i === 0 ? { x: p.x - 30, y: p.y - 20 } : p)));
    poseTo({ ...flatCam(), pitch: 16 });
    expect(S.cam.pitch).toBe(16);
    expect(S.base).toBe(S.src);

    poseTo(flatCam());
    S.dst.forEach((p, i) => {
      expect(p.x).toBeCloseTo(S.src[i].x, 9);
      expect(p.y).toBeCloseTo(S.src[i].y, 9);
    });
    expect(S.cam).toEqual(flatCam());
  });

  it("keeps the anchors on reset", () => {
    const src = S.src.map((p) => ({ x: p.x + 10, y: p.y + 5 }));
    placeTo(src, homography(S.goodSrc, S.good));
    poseTo(flatCam());
    expect(S.src).toBe(src);
  });
});

describe("unexported work", () => {
  it("is clean when a photo opens, even one opened with a shift", () => {
    expect(isDirty()).toBe(false);
    openPhoto("photo.jpg", "sample.jpg", W, H, { ...flatCam(), yaw: 12 });
    expect(isDirty()).toBe(false);
  });

  it("is dirty after a shift, and clean again after undo", () => {
    remember(null);
    setPoint(0, moved(0, 30));
    expect(isDirty()).toBe(true);
    undo();
    expect(isDirty()).toBe(false);
  });

  it("is clean after an export, until the next change", () => {
    setPoint(0, moved(0, 30));
    markSaved();
    expect(isDirty()).toBe(false);
    setPoint(0, moved(0, 30));
    expect(isDirty()).toBe(true);
  });

  it("is not affected by a crossed shape that snapped back", () => {
    shiftTo([S.dst[2], S.dst[1], S.dst[0], S.dst[3]]);
    expect(isDirty()).toBe(false);
    settle();
    expect(isDirty()).toBe(false);
  });

  it("is clean with no photo open", () => {
    set({ view: "start" });
    expect(isDirty()).toBe(false);
    set({ view: "editor" });
  });
});
