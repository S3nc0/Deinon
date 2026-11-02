# Test by hand

What automated tests cannot confirm.

## Already checked on macOS

During the build, the built interface was compared with `design/index.html` in WebKit, with the native side mocked. A test build also ran in the real window, where these worked: opening the sample photo through Rust, fonts, the content security policy, the permission refusals, and the save dialog appearing. **Nobody has yet clicked Save in that dialog and exported through the real window.** The export code is covered by `cargo test`, but start with the Export section below.

## Open photos

- [ ] **Open photo** dialog shows only JPEG, PNG, TIFF and WebP
- [ ] Each format opens. TIFF shows correctly
- [ ] A phone photo taken in portrait opens upright
- [ ] Dropping an image opens it, and the stage or drop area highlights during the drag
- [ ] Dropping a text file shows the error
- [ ] A renamed text file (`notes.jpg`) shows an error and does not crash

## Editing

- [ ] Drag a point with the mouse: the photo warps live and the sliders follow
- [ ] Drag inside the shape: all 4 points move
- [ ] Drag a point across another: red edges, then it snaps back on release
- [ ] Place points: hollow dots and the loupe while dragging
- [ ] Compare: the divider drags
- [ ] A photo over 20 megapixels stays smooth while dragging

## Export

- [ ] Export each format, then open the file in another app
- [ ] The exported image matches the preview
- [ ] A wide-gamut photo keeps its colours (check the profile in Preview or Photoshop)
- [ ] Choose a file name that exists: the dialog asks about replacing it
- [ ] Choose the original photo as the target: refused, and the file is unchanged
- [ ] Cancel a large export: no file appears in the folder, including hidden files
- [ ] Save to a read-only folder: a clear error, and the app keeps working

## Menus and windows

- [ ] Every File, Edit and View item works, and its shortcut does too
- [ ] Undo and Redo grey out with the toolbar buttons
- [ ] Compare greys out in Place points
- [ ] In the file name field: `Cmd+Z`, `Cmd+C`, `Cmd+V`, `Cmd+X` and `Cmd+A` act on the text
- [ ] The window title shows the file name
- [ ] Theme and window size return after a restart
- [ ] With an unexported shift, opening another photo, `Cmd+Q` and the close button all ask first
- [ ] Quit from the Dock with an unexported shift (known limit: it may not ask)

## Windows and Linux

The release workflow builds these and runs the automated tests on them. Nobody has opened the app on them yet.

- [ ] The release `.exe` and `.msi` install on a clean PC, and WebView2 downloads if it is missing
- [ ] All the checks above, using `Ctrl`
- [ ] `Ctrl+C` and `Ctrl+V` work in text fields
- [ ] The drop highlight works
- [ ] On Linux, the `.deb` and `.AppImage` run

## macOS downloads

- [ ] The release `.dmg` opens on another Mac after using **Open Anyway**, on both Apple Silicon and Intel
