# Architecture

## Modules

| Module | Job |
| --- | --- |
| `src/engine/` | Pure maths with no DOM: `homography`, `validity`, `coverage`, `camera`, `crop` |
| `src/native/api.ts`, `menu.ts` | The only files that call Tauri |
| `src/ui/state.ts` | The single state object `S`, `set()` and the render schedule |
| `src/ui/history.ts` | Undo and redo: 50 steps, grouped within 700 ms |
| `src/ui/edits.ts` | Every change to the points and camera, presets, and the unexported check |
| `src/ui/drag.ts`, `points.ts` | Pointer drags, and building the points and their cards |
| `src/ui/render.ts`, `render-stage.ts`, `render-panel.ts` | Draw `S` into the DOM |
| `src/ui/commands.ts` | The code behind each toolbar button and menu item |
| `src/ui/controls.ts` | Connects the DOM controls to that code |
| `src/ui/desktop.ts` | Menu events, file drops, and reporting state to the native side |
| `src/ui/open-photo.ts`, `export-panel.ts` | The open and export flows |
| `src/ui/output-size.ts` | The Output and gaps values in the panel |
| `src-tauri/src/warp.rs` | The same maths as the engine, for export |
| `src-tauri/src/photo.rs` | Decoding, orientation, colour profile, preview |
| `src-tauri/src/export.rs` | Rendering, encoding, safe writing |
| `src-tauri/src/commands.rs` | Tauri commands, dialogs, window events |
| `src-tauri/src/state.rs` | The open photo and the paths the person chose |
| `src-tauri/src/menu.rs` | The native menu |

## Opening a photo

1. **Open:** the toolbar or menu runs `commands.open`, then `requestOpen`. Rust's `choose_photo` shows the dialog and remembers the path. A drop reaches Rust as a window event and is remembered the same way, then arrives as a `file-drop` event.
2. `open_photo` checks that the path was chosen, then decodes it on a background thread.
3. Rust keeps the full-resolution pixels in `AppState` and returns the original size, the name and a JPEG preview of at most 2560 px.
4. `edits.openPhoto` sets up `S`. All positions are in original photo pixels, and the preview is stretched to the original's size in CSS.

## Editing

An input calls a function in `edits.ts`, which updates `src` and `dst`, plus `goodSrc` and `good` when the shape is valid. `set()` schedules one render per frame. The photo is drawn with `homography(goodSrc, good)` as a CSS `matrix3d`. Rust is not involved.

After each render, `syncDesktop` sends the title, menu states and unexported flag to `set_ui_state`, if they changed. Rust uses that flag to ask before quitting.

## Exporting

1. `runExport` calls `choose_export_path`. Rust shows the save dialog and remembers the path, once.
2. `export_photo` receives `goodSrc`, `good`, the path and the options, and rejects anything invalid.
3. `export::render` computes the same homography, then for each output pixel maps backwards through its inverse and samples the original. It runs across cores with `rayon`, sends `export-progress` events and checks a cancel flag on every row.
4. `export::encode` writes to a temporary file in the target folder, which is renamed when complete.

## Keeping preview and export the same

`scripts/make-fixtures.mjs` runs the maths in `design/index.html` to write `fixtures/homography.json`. `src/engine/engine.test.ts` and `src-tauri/src/warp.rs` both test against it, within 1e-6.

## Tests

- **Vitest (`src/**/*.test.ts`):** engine, history, edits, output size, and the API layer with Tauri mocked.
- **`cargo test`:** maths, decoding, rendering, encoding, safe writing, and `tests/open_to_export.rs` for the whole path through Rust.
