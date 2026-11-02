# Deinon: GUI design

Deinon changes a photo's first-person viewpoint using 4 points. You pin the points to the photo, then move them, and the whole photo warps to follow. This file covers what the GUI does and how it behaves. For tokens, component specs and the maths, see [system.md](system.md).

Working reference: [index.html](index.html) with [design.css](design.css). Open `index.html` directly in a browser, no server needed.

## Core idea

Each point has two positions:

| Position | Meaning | Edited with |
| --- | --- | --- |
| Anchor | The pixel on the original photo the point is pinned to | Place points tool |
| Target | Where that pixel should land in the output frame | Shift view tool |

The 4 anchor and target pairs fully define one perspective warp. Anything else that changes the view, such as a camera slider or a preset, works by moving the targets.

## Workflow

1. **Open a photo.** Each point starts as an anchor 20% in from the edges, with no shift applied.
2. **Place points (optional).** Pin the points to straight features in the photo.
3. **Shift the view.** Drag points, use the virtual camera, or pick a preset.
4. **Export.** Choose the format, size and what to do with empty areas.

## Screen layout

```
+------------------------------------------------------------------+
| Deinon                             [open] [export] [theme]       |  48 px header
+-----------------------------------------------+------------------+
| [Place points | Shift view]  undo redo reset  | [Shift | Export] |  Tool bar / tabs
| compare mesh grid              zoom - 58% + fit+------------------+
+-----------------------------------------------+ Points           |
|                                               |  [1]      [2]    |
|   Output frame                                |  [4]      [3]    |
|   +---------------------------------------+   +------------------+
|   |   1 o-----------------------o 2       |   | Virtual camera   |  380 px panel
|   |     |       photo, warped   |         |   |  Yaw   Pitch     |
|   |     |                       |         |   |  Roll  FOV       |
|   |   4 o-----------------------o 3       |   +------------------+
|   +---------------------------------------+   | Presets          |
|                                               |  chips           |
+-----------------------------------------------+                  |
| file.jpg  1600 x 1067 px  hint       [Gaps 6%]|                  |  Status bar
+-----------------------------------------------+------------------+
```

The point cards are arranged like the corners of the photo: 1 and 2 on top, 4 and 3 below, so the panel maps directly onto the stage.

## Screens

### Start

- A dashed drop area with a tilted 4-point shape that shows what the tool does.
- Heading: "Open a photo to shift its view".
- Main action: **Open photo**. Second action: **Use the sample photo**.
- Three short steps below: Place 4 points, Shift the view, Export.
- Dropping an image anywhere on the drop area opens it.

### Shift view (default tool)

- The photo warps live as points move.
- Dragging inside the shape slides all 4 points together.
- A dashed line and a small ring show where each point started.
- The status bar reports gaps in the frame.

### Place points

- The photo is shown unwarped, so you can see what you are pinning to.
- Dots turn hollow so the pixel underneath stays visible.
- A loupe appears while dragging, with a crosshair and the exact pixel position.
- Moving an anchor does not change the current warp. The target follows, so the shift you already made stays exactly as it was.

### Export tab

| Control | Options | Rule |
| --- | --- | --- |
| File name | Text | The extension is added on export |
| Format | JPEG, PNG, TIFF, WebP | |
| Size | Original, or a long edge of 3840, 2560, 1920 or 1280 px | Keeps the aspect ratio |
| Quality | 40 to 100% | Only shown for JPEG and WebP |
| Empty areas | Crop to the photo, Stretch the edges, Keep transparent | Transparent is disabled for JPEG. Switching to JPEG moves it to Crop |

A summary shows the output size and the gaps in the frame.

## Interactions

| Input | Result |
| --- | --- |
| Drag a point | Moves that point. The sliders update to match |
| Drag inside the shape | Moves all 4 points together |
| Arrow keys on a point | Nudges by 1 px, or 10 px with Shift |
| X or Y field | Types an exact position. Arrow up or down steps by 1, or 10 with Shift |
| Yaw, Pitch, Roll sliders | Turn the virtual camera. All 4 points move together |
| Field of view slider | Zooms around the centre. 84° means no zoom |
| Preset chip | Animates to a set camera pose over 240 ms, measured from the unshifted photo |
| Reset | Animates back to no shift. Anchors stay where they are |
| Undo, Redo | Up to 50 steps. One drag is one step, and repeated slider or key changes within 700 ms are grouped |
| Compare | Shows the original to the right of a draggable divider and hides the points |
| Mesh | Toggles the perspective grid inside the shape |
| Thirds grid | Toggles a rule of thirds grid on the output frame |
| Zoom | 50% to 400% of the fitted size, in 25% steps |

### Camera direction

| Slider | Positive value |
| --- | --- |
| Yaw | Look right: the right side of the photo gets smaller |
| Pitch | Look up: the top of the photo gets smaller |
| Roll | Clockwise tilt |

## States

### Points

| State | Appearance |
| --- | --- |
| Rest | 14 px solid dot and a number tag |
| Hover | The full 44 px target tints |
| Keyboard focus | 2 px focus line with an inset gap |
| Selected or dragging | 18 px dot, inverted tag, and the matching card highlights in the panel |
| Placing | Hollow dot, plus the loupe while dragging |
| Crossed | Red dot, tag and edges |

### Crossed points

The 4 points must always form a convex four-sided shape. If a move breaks that, or a warp becomes too extreme:

1. The edges and the selected point turn red.
2. The photo holds its last good shape.
3. The status bar shows "Points crossed" and says which point to move back.
4. When you let go of the point, it snaps back to the last good position. A camera slider stays where it is until you ease it back.

### Frame coverage

| Status tag | Meaning |
| --- | --- |
| Frame full | The photo covers the whole frame |
| Gaps 6.2% | Part of the frame is empty, shown as a checkerboard |
| 4 points placed | Shown in the Place points tool |
| Points crossed | The shape is invalid |

## Writing style

- Sentence case everywhere.
- Buttons start with a verb: "Open photo", "Export photo".
- Errors say how to fix the problem.
- Numbers carry real units: px, °, %.

## Small windows

Below 880 px wide:

- The side panel stacks under the stage and the page scrolls.
- The editor area has a fixed height of 600 px.
- The tool switch takes the full width, and the other toolbar buttons wrap below it.

## Accessibility

- Points are real buttons, usable with the keyboard alone, and they announce their number and position.
- Every icon-only button has an accessible name and a tooltip. Toggles use `aria-pressed`.
- Status is never colour alone: each tag has an icon and a word.
- Hints and status changes use `aria-live`.
- All touch and pointer targets on the stage are 44 px.
- Reduced motion turns off the preset animation.

## What is design only

This copy of the design describes the built app, so little is left here:

- **Export** writes a real file through the renderer in `src-tauri/src/export.rs`, and the empty area options take effect. The preview always shows gaps as a checkerboard, whichever option is chosen.
- **Photo formats:** JPEG, PNG, TIFF and WebP open on every platform, because Rust decodes them. Camera raw files are refused with an error that says what to open instead.
- **Panning** the stage is not designed yet. Zoom always stays centred.
