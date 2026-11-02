# FPV Shifter: design system and shift engine

This file is the reference for building the GUI: tokens, components, stage layers and the maths behind the warp. For what the product does and why, see [design.md](design.md).

## Files

| File | Holds |
| --- | --- |
| [design.css](design.css) | Tokens and every component style. Has no dependencies except Google Fonts |
| [index.html](index.html) | The working editor in plain HTML and JavaScript |
| `assets/sample.jpg` | A computer-generated sample photo. Replace it with your own |

## Foundation

The visual language comes from the Leafy design system: Verdant M3 colour roles with Carbon type, shape and density.

**The rule:** colour and behaviour follow Material 3. Geometry and density follow Carbon. Colours are assigned by role and used in pairs, a fill plus its `on-` colour.

`design.css` is a self-contained subset. It uses plain class names (`.btn`, `.chip`), not Leafy's `lf-` classes, and it includes only the Dark and Light themes. Leafy's two high-contrast themes are left out.

## Colour

Set the theme with `data-theme="dark"` or `data-theme="light"` on any element. Dark is the default because neutral, dark surroundings suit photo work.

| Token | Dark | Light | Used for |
| --- | --- | --- | --- |
| `--primary` | `#a1d39a` | `#3b6939` | Points, edges, main button, slider fill |
| `--on-primary` | `#0a390f` | `#ffffff` | Text and rings on primary |
| `--primary-container` | `#235024` | `#bcf0b4` | "Shifted" compare tag |
| `--secondary-container` | `#3b4b38` | `#d5e8cf` | Selected segment and point card |
| `--on-secondary-container` | `#d5e8cf` | `#3b4b38` | Text on selection |
| `--error` | `#ffb4ab` | `#ba1a1a` | Crossed points and edges |
| `--on-error` | `#690005` | `#ffffff` | Text and rings on error |
| `--surface` | `#10140f` | `#f7fbf1` | Page, header |
| `--surface-container-lowest` | `#0b0f0a` | `#ffffff` | Stage background |
| `--surface-container` | `#1d211b` | `#ecefe6` | Checkerboard, first tone |
| `--surface-container-high` | `#272b25` | `#e6e9e0` | Checkerboard, second tone. Loupe |
| `--on-surface` | `#e0e4db` | `#191d17` | Main text |
| `--on-surface-variant` | `#c2c9bd` | `#424940` | Secondary text, resting icons |
| `--layer-01` | `#191d17` | `#f1f5ec` | Panel, toolbar, status bar |
| `--layer-02` | `#1d211b` | `#ecefe6` | Point cards, summary box |
| `--layer-accent` | `#323630` | `#e0e4db` | Inactive tab |
| `--field-01` | `#323630` | `#e0e4db` | Inputs |
| `--border-subtle` | `#424940` | `#c2c9bd` | Dividers |
| `--border-strong` | `#8c9388` | `#72796f` | Input underline |
| `--outline` | `#8c9388` | `#72796f` | Frame outline, segment border |
| `--inverse-surface` | `#e0e4db` | `#2d322c` | Selected point, tooltip, divider |
| `--focus` | `#a0cfd4` | `#38656a` | Focus line |
| `--focus-inset` | `#10140f` | `#f7fbf1` | Gap inside the focus line |
| `--scrim` | `#000000` | `#000000` | Dim outside the frame, line shadows |

Status tags use `--success-container`, `--warning-container` and `--error-container`, each with its `on-` pair.

**Colour on top of photos:** white is used only for the thirds grid, the start-position markers and the "Output frame" label. Every point and edge has a dark line beneath it, so it reads on bright and dark photos alike.

## Type

IBM Plex Sans for the interface. IBM Plex Mono for machine values: pixel positions, file names, slider values and zoom. Icons are Material Symbols Sharp at weight 300.

| Style | Size / line height | Weight | Tracking | Used for |
| --- | --- | --- | --- | --- |
| Headline | 28 / 36 | 400 | 0 | Start screen heading |
| Section title | 14 / 18 | 600 | 0.16 px | "Points", "Virtual camera" |
| Body | 14 / 20 | 400 | 0.16 px | Default text |
| Label | 14 / 18 | 500 | 0.16 px | Buttons, tabs, chips |
| Caption | 12 / 16 | 400 | 0.32 px | Field labels, status bar |
| Point number | 11 / 18 | 600 mono | 0 | Number tags |
| Value | 12 / 16 | 400 mono | 0.32 px | Coordinates, slider values |

## Space, size and shape

- **Spacing:** `--space-half` (2), `-1` (4), `-2` (8), `-3` (12), `-4` (16), `-5` (20), `-6` (24), `-8` (32), `-12` (48), `-16` (64).
- **Control heights:** `--size-xs` (24), `--size-sm` (32), `--size-md` (40), `--size-lg` (48).
- **Corners:** `--corner-none` (0), `--corner-xs` (2), `--corner-sm` (4), `--corner-full`.
- **Round shapes** are only for selection and status: points, chips, tags, slider handles, radios. Everything else is crisp.
- **Depth:** flat layers first. Only the loupe and the divider knob cast a shadow.

## Motion and states

- **Easing:** `--ease-productive`, `cubic-bezier(0.2, 0, 0.38, 0.9)`.
- **Durations:** `--fast` (110 ms) for state changes, `--moderate` (150 ms) for tooltips, 240 ms for preset animation.
- **Hover:** content colour at 8%. **Pressed:** 12%.
- **Disabled:** content at 38%, with a not-allowed cursor.
- **Focus:** 2 px `--focus` line inside the control, with a 3 px `--focus-inset` gap.
- **Selected:** `--secondary-container`, plus a 3 px `--primary` bar on point cards.

## Components

| Class | Spec |
| --- | --- |
| `.btn--filled` | 40 px. The one main action in a view |
| `.btn--outlined` | 40 px. Second action |
| `.icon-btn` | 40 px square, 48 px in the header. `data-tip` gives a tooltip, `aria-pressed` makes it a toggle |
| `.seg` / `.seg__btn` | 40 px. Selected segment shows a check |
| `.tabs` / `.tab` | 48 px contained tabs. Selected gets a 2 px primary top line |
| `.field` / `.field__input` | 40 px stacked input. `.field--sm` is 32 px and mono |
| `.slider` | 2 px track, 16 px handle, value box beside it. Set `--fill` for WebKit browsers |
| `.chip` | 32 px pill with a 16 px primary icon |
| `.tag--*` | 24 px pill: `neutral`, `primary`, `success`, `warning`, `error` |
| `.radio` | Native radio with a primary accent |
| `.notice` | Inline success message with a 3 px bar |

### Point (`.point`)

| Part | Spec |
| --- | --- |
| Target | 44 × 44 px real `<button>`, centred on the position |
| Dot | 14 px at rest, 18 px selected, with a 2 px ring and a 1 px scrim line |
| Number tag | 18 px square, 2 px corners, placed outward from its corner using `data-corner` |
| Edge | 1.5 px over a 3.5 px scrim line at 55% |
| Mesh | 1 px primary at 85% over a 2.5 px scrim line at 45%, 3 lines each way |
| Start marker | 7 px white ring, with a dashed line to the point |

State classes: `.is-selected` and `.is-dragging` on the point. `.tool-place` and `.is-invalid` on the stage.

### Loupe (`.loupe`)

144 px square view with a 1 px primary crosshair and a label below. Magnification is 4 times the stage scale, and never below 2× the photo's pixels. It sits above and to the right of the point, and flips when it would leave the stage.

## Stage layers

From back to front, inside `.stage`:

1. `.frame-checker`: checkerboard at the output frame
2. `.photo`: the warped image, at natural size with `transform-origin: 0 0`
3. `.frame-mask`: a 4000 px spread shadow at 62% scrim that dims everything outside the frame
4. `.frame-label`: "Output frame"
5. `.compare-photo`: the original, clipped to the right of the divider
6. `.overlay`: SVG with the thirds grid, mesh, start lines and edges
7. `.quad-grab`: full-size layer clipped to the shape, for dragging all 4 points
8. `.ghost`: start markers
9. `.point`: the 4 point buttons
10. `.divider-grip`, `.compare-tag`, `.loupe`

The stage keeps a 64 px margin around the frame, or 32 px below 880 px wide, so points can leave the frame and still be grabbed.

## Shift engine

All positions are in photo pixels, with (0, 0) at the top left of the frame. Points run clockwise: 1 top left, 2 top right, 3 bottom right, 4 bottom left.

### State

| Value | Meaning |
| --- | --- |
| `src[4]` | Anchors on the photo |
| `dst[4]` | Targets in the frame, as shown. May be invalid during a drag |
| `goodSrc[4]`, `good[4]` | The last valid pair. This is what the photo is drawn with |
| `cam` | Current yaw, pitch, roll and field of view |
| `base[4]`, `camBase` | The targets and camera that slider changes are measured from |

### The warp

A homography `H` maps each anchor to its target:

```
H = Q(dst) × inverse(Q(src))
```

`Q(q)` is the standard square-to-quad matrix (Heckbert) that maps the unit square onto the 4 points. See `squareToQuad` in `index.html`. Normalise `H` so its last element is 1.

To map a point:

```
w  = h6·x + h7·y + h8
x' = (h0·x + h1·y + h2) / w
y' = (h3·x + h4·y + h5) / w
```

To draw it in CSS, where `s` is the stage scale:

```
transform: scale(s) matrix3d(h0, h3, 0, h6,  h1, h4, 0, h7,  0, 0, 1, 0,  h2, h5, 0, 1);
```

### Virtual camera

The camera treats the photo as a plane rotating about its centre:

```
f = (width / 2) / tan(42°)

Roll:   x1 = x·cos r − y·sin r         y1 = x·sin r + y·cos r
Pitch:  y2 = y1·cos p                  z2 = −y1·sin p
Yaw:    x3 = x1·cos a − z2·sin a       z3 = x1·sin a + z2·cos a
Zoom:   k  = f / (f + z3) × tan(fov0 / 2) / tan(fov / 2)

x' = cx + x3·k      y' = cy + y2·k
```

The base lens is 84°, a wide first-person view. Slider changes apply the difference between `cam` and `camBase` to `base`, so they build on any hand-dragged shape.

### Reading the camera back

After a manual drag, map the photo corners through `H`, then measure left height `hL`, right height `hR`, top width `wT` and bottom width `wB`:

```
sin(yaw)   = (hL − hR) / (hL + hR) × 2f / width
sin(pitch) = (wB − wT) / (wB + wT) × 2f / height
roll       = mean angle of the top and bottom edges
scale      = sqrt(area of mapped corners / photo area)
fov        = 2 · atan(tan(42°) / scale)
```

This is exact when one axis moves alone and a close estimate for combinations.

### Validity

A pair is valid only if all of these hold:

- `src` and `dst` are both convex.
- They wind in the same direction, so the photo is not flipped.
- `w > 0.05` at all 4 photo corners, so nothing passes behind the camera.

### Coverage

Map the photo corners through `H`, clip that shape to the frame with Sutherland-Hodgman, and divide its area by the frame's. Gaps are `1 − coverage`. Below 0.05% counts as "Frame full".

### Building the real export

Export is not implemented. CSS transforms are preview only, so the final image needs its own render:

1. Create an output buffer at the export size.
2. For each output pixel, scale back to frame coordinates and map through `inverse(H)` to find its source position. Mapping backwards leaves no holes.
3. Sample the original at full resolution with bilinear filtering, or Lanczos for better quality.
4. Handle pixels that fall outside the photo using the chosen empty-area option:
   - **Keep transparent:** alpha 0.
   - **Stretch the edges:** clamp the source position to the photo.
   - **Crop to the photo:** find the largest rectangle inside the warped photo, then crop to it.
5. Encode, keeping the colour profile and metadata from the original.

WebGL or WebGPU suits a browser build. OpenCV's `getPerspectiveTransform` and `warpPerspective` do the same job in Python or C++.
