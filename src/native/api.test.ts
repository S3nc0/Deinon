import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
const listen = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const api = await import("./api.ts");
const { onMenuCommand } = await import("./menu.ts");

beforeEach(() => {
  invoke.mockReset();
  listen.mockReset();
});

describe("commands", () => {
  it("calls each native command with the right arguments", async () => {
    invoke.mockResolvedValue(null);
    await api.choosePhoto();
    expect(invoke).toHaveBeenLastCalledWith("choose_photo");
    await api.openPhoto("/photos/a.jpg");
    expect(invoke).toHaveBeenLastCalledWith("open_photo", { path: "/photos/a.jpg" });
    await api.openSample();
    expect(invoke).toHaveBeenLastCalledWith("open_sample");
    await api.chooseExportPath("a_shifted", "webp");
    expect(invoke).toHaveBeenLastCalledWith("choose_export_path", { fileName: "a_shifted", format: "webp" });
    await api.cancelExport();
    expect(invoke).toHaveBeenLastCalledWith("cancel_export");
    await api.confirmDiscard();
    expect(invoke).toHaveBeenLastCalledWith("confirm_discard");
  });

  it("sends the export request in the shape Rust expects", async () => {
    invoke.mockResolvedValue({ fileName: "a.png", width: 10, height: 20 });
    const request = {
      src: [{ x: 0, y: 0 }],
      dst: [{ x: 1, y: 1 }],
      path: "/photos/a.png",
      options: { format: "png" as const, longEdge: null, quality: 90, empty: "crop" as const },
    };
    await expect(api.exportPhoto(request)).resolves.toEqual({ fileName: "a.png", width: 10, height: 20 });
    expect(invoke).toHaveBeenCalledWith("export_photo", { request });
  });

  it("passes on a refusal from Rust", async () => {
    invoke.mockRejectedValue({ code: "error", message: "This file isn't a supported image." });
    await expect(api.openPhoto("/a.txt")).rejects.toEqual({ code: "error", message: "This file isn't a supported image." });
  });
});

describe("events", () => {
  const fire = <T>(payload: T) => {
    const handler = listen.mock.calls[0][1] as (e: { payload: T }) => void;
    handler({ payload });
  };

  it("reports export progress", () => {
    const seen: number[] = [];
    void api.onExportProgress((f) => seen.push(f));
    expect(listen.mock.calls[0][0]).toBe("export-progress");
    fire({ fraction: 0.5 });
    expect(seen).toEqual([0.5]);
  });

  it("reports file drops", () => {
    const seen: unknown[] = [];
    void api.onFileDrop((d) => seen.push(d));
    expect(listen.mock.calls[0][0]).toBe("file-drop");
    fire({ kind: "drop", paths: ["/a.jpg"] });
    expect(seen).toEqual([{ kind: "drop", paths: ["/a.jpg"] }]);
  });

  it("reports menu commands", () => {
    const seen: string[] = [];
    void onMenuCommand((id) => seen.push(id));
    expect(listen.mock.calls[0][0]).toBe("menu-command");
    fire("undo");
    expect(seen).toEqual(["undo"]);
  });
});

describe("errors", () => {
  it("reads the message from an app error", () => {
    expect(api.errorMessage({ code: "error", message: "Choose another name." })).toBe("Choose another name.");
    expect(api.errorMessage("Plain text")).toBe("Plain text");
    expect(api.errorMessage(undefined)).toContain("Something went wrong");
  });

  it("tells a cancel from a failure", () => {
    expect(api.isCancelled({ code: "cancelled", message: "Export cancelled." })).toBe(true);
    expect(api.isCancelled({ code: "error", message: "No" })).toBe(false);
    expect(api.isCancelled(null)).toBe(false);
  });
});

describe("previewUrl", () => {
  it("turns base64 into an image blob URL", async () => {
    const created: Blob[] = [];
    vi.stubGlobal("URL", { createObjectURL: (b: Blob) => (created.push(b), "blob:preview") });
    expect(api.previewUrl(btoa("\xff\xd8\xff"))).toBe("blob:preview");
    expect(created[0].type).toBe("image/jpeg");
    expect(Array.from(new Uint8Array(await created[0].arrayBuffer()))).toEqual([0xff, 0xd8, 0xff]);
    vi.unstubAllGlobals();
  });
});
