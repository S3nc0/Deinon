import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const { firstSupported } = await import("./open-photo.ts");

describe("firstSupported", () => {
  it("picks the first image the app can open", () => {
    expect(firstSupported(["/a/notes.txt", "/a/b.JPG", "/a/c.png"])).toBe("/a/b.JPG");
    for (const name of ["a.jpg", "a.jpeg", "a.png", "a.tif", "a.tiff", "a.webp"]) {
      expect(firstSupported([name])).toBe(name);
    }
  });

  it("returns null when nothing is supported", () => {
    expect(firstSupported(["/a/notes.txt", "/a/movie.mov", "/a/raw.cr3", "/a/jpg"])).toBeNull();
    expect(firstSupported([])).toBeNull();
  });
});
