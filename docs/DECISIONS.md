# Decisions

Choices made during the build that the design did not specify.

## Build

- **Built by hand.** The original docs describe unattended build scripts. The app was built directly instead, following the same phases and checks.
- **Name.** The app began as FPV Shifter, and versions up to 0.1.1 were released under that name. It is now Deinon, with the icon in `src-tauri/icons/source.svg`.
- **Bundle identifier.** `io.github.s3nc0.deinon`, based on the GitHub account so that it is unique. It replaced `com.fpvshifter.app`, so settings from 0.1.1 and earlier do not carry over and the new app installs alongside the old one. Avoid changing it again, for the same reason.
- **Publisher.** `Deinon` is a placeholder.
- **Releases.** The original plan built only Windows installers and published nothing. On request, `.github/workflows/release.yml` replaces that workflow. It builds macOS (Apple Silicon and Intel), Windows and Linux, and publishes a release only after all of them pass.
- **Ad-hoc signing on macOS.** The app is signed with no developer identity (`signingIdentity: "-"`). macOS reports an unsigned download on Apple Silicon as damaged, which is hard to get past. With an ad-hoc signature, it shows the normal warning and **Open Anyway** works. This is not a replacement for a Developer ID and notarization.
- **Windows manifest.** `src-tauri/build.rs` embeds the app manifest through the linker instead of using Tauri's default, which only reaches the app. Without this, Rust test programs can't start on Windows (`STATUS_ENTRYPOINT_NOT_FOUND`).
- **Screenshots** in `docs/images/` were taken from the built interface in WebKit, using the computer-generated sample photo.

## Security

- **Dialogs run in Rust, not the webview.** The prompts ask for `tauri-plugin-dialog`, which is used, but through its Rust API. The webview therefore has no dialog permission. Its only capabilities are listening for events.
- **Commands only touch chosen files.** `open_photo` accepts only paths that came from the open dialog or a drop. `export_photo` accepts only the path just returned by the save dialog, once. A compromised webview cannot read or write anything else.
- **Inline style removed.** The reference has one `style="margin: 0"` attribute, which the content security policy would block. It is now a rule in `src/styles/app.css`.
- **WebKit reports one policy violation at load**, from the internal markup of the `<select>` element. It is not app code, and the control renders the same as the reference. Allowing `unsafe-inline` to silence it would weaken the policy.

## Styles

- `src/styles/design.css` is the design stylesheet with only the Google Fonts import removed. Everything the design has no style for is in `src/styles/app.css`: the error banner, loading state, export progress, export error and disabled button. These use the design's tokens.

## Opening photos

- **Sample photo.** It is built into the Rust binary and opens through the same path as any photo, so export works on it.
- **Limit.** Photos over 400 megapixels are refused with a clear error. At 4 bytes a pixel, that is 1.6 GB of memory.
- **8-bit working copy.** Every photo is converted to 8-bit RGBA when opened, so **16-bit PNG and TIFF files lose their extra precision on export.** Keeping 16 bits would need a second render path.
- **Transparent photos** show on white in the preview, because the preview is a JPEG. Export keeps their alpha, except in JPEG, which flattens onto white.
- **Drop area.** A drop anywhere in the window opens the photo, and the stage or drop area highlights for the whole drag. The reference only accepts drops on those two areas. Coordinates from native drop events differ between platforms, so this is the more reliable choice.
- **Order of prompts.** The discard prompt appears after you pick a file, so cancelling the picker never asks a question.

## Export

- **Crop size.** "Crop to the photo" at Original size outputs the crop at 1:1, so it is smaller than the frame. With a long edge chosen, the crop is scaled to reach that long edge. The Output line in the panel shows the true size either way, which is why `cropRect` also exists in TypeScript.
- **Crop position.** The largest rectangle can sometimes slide, so the middle of its possible positions is used.
- **Shrinking.** Bilinear sampling alone aliases when shrinking, so each output pixel takes up to 4 × 4 samples when it spans more than one photo pixel.
- **Hard edges.** With Keep transparent at 1:1 or larger, the photo's edge is a hard line, as specified: alpha is 0 outside the photo. Shrinking softens it.
- **Saving.** A file is written to a temporary name in the target folder, then renamed. Failed and cancelled exports leave nothing behind.
- **Extension.** If the save dialog returns a name without the format's extension, it is added. If that new name already exists it is refused, because the dialog never asked about replacing it.
- **WebP.** Lossy through the `webp` crate, so the quality slider works. That encoder drops metadata, so the app adds the colour profile and EXIF back itself. WebP is limited to 16383 px a side.

### What each format keeps

| Format | Colour profile | EXIF | XMP and other metadata |
| --- | --- | --- | --- |
| JPEG | Yes | Yes | No |
| PNG | Yes | Yes | No |
| TIFF | Yes | No, the encoder cannot write it | No |
| WebP | Yes | Yes | No |

EXIF is copied as it was, with the orientation cleared because the pixels are already upright. Tags such as the original pixel size are not updated. EXIF in a TIFF source is not read. GPS data in EXIF is kept, as the design asks for metadata to be kept.

## Desktop

- **Menu additions.** macOS needs an app menu and Cut, Copy, Paste and Select All items, or text fields lose those shortcuts. They are added on macOS only.
- **Shortcuts.** The design names no keys for these: Export `Cmd+E`, Reset `Cmd+Shift+R`, Fit `Cmd+0`, Compare `Cmd+Shift+C`, Mesh `Cmd+Shift+M`, Grid `Cmd+Shift+G`, Theme `Cmd+Shift+T`. Single letters would interfere with typing.
- **File > Export** opens the Export tab, the same as the header button, because the settings live there.
- **Menu undo in text fields.** Native shortcuts reach the menu before the webview, so Undo and Redo are handed back to a focused text field. The menu items stay enabled while one is focused.
- **Zoom at its limits.** Menu items stay enabled at 50% and 400%, matching the reference's toolbar buttons.
- **Theme** is stored in the webview's local storage, which is simple and readable before the first paint. **Window size** uses the window-state plugin. Position is not remembered, as only size was asked for.
- **Unexported work** means the warp differs from the one at opening or the last export. Moving anchors alone keeps the warp, so it does not count, and undoing back to the start is clean.

## Fixes to the reference

- Moving a slider, typing a position or nudging a point during a preset's 240 ms animation now finishes the animation first. In the reference, the animation overwrote that change.

## Known limits

- **16-bit photos** export at 8 bits, as above.
- **Quit from the Dock** on macOS may skip the unexported work prompt. The menu, `Cmd+Q` and the close button all ask.
- **Quitting during an export** can leave a hidden `.deinon-*.tmp` file in the target folder, if it happens during encoding.
- **Very large previews.** The preview is drawn at the original's CSS size, as the design asks. Behaviour with the largest photos should be checked by hand.
