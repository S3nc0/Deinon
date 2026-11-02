import { homography, type Matrix, type Quad } from "../engine/index.ts";
import { $, clamp } from "./dom.ts";
import { cancelTween, placeTo, settle, shiftTo } from "./edits.ts";
import { remember } from "./history.ts";
import { geom, S, set } from "./state.ts";

type DragKind = "point" | "quad" | "split";

interface Drag {
  kind: DragKind;
  idx: number;
  r: DOMRect;
  x0: number;
  y0: number;
  src0: Quad;
  dst0: Quad;
  M0: Matrix;
  moved: boolean;
}

let drag: Drag | null = null;

export function startDrag(e: PointerEvent, kind: DragKind, idx = 0) {
  if (e.button) return;
  cancelTween();
  drag = {
    kind,
    idx,
    r: $("stage").getBoundingClientRect(),
    x0: e.clientX,
    y0: e.clientY,
    src0: S.src,
    dst0: S.dst,
    M0: homography(S.goodSrc, S.good),
    moved: false,
  };
  window.addEventListener("pointermove", dragMove);
  window.addEventListener("pointerup", dragEnd);
  window.addEventListener("pointercancel", dragEnd);
  set(kind === "point" ? { dragging: true, selected: idx } : { dragging: false });
}

function dragMove(ev: PointerEvent) {
  if (!drag) return;
  const g = geom;
  const sx = ev.clientX - drag.r.left;
  const sy = ev.clientY - drag.r.top;
  if (drag.kind === "split") {
    set({ split: clamp((sx - g.fx) / g.fw, 0, 1) });
    return;
  }
  if (!drag.moved) {
    drag.moved = true;
    remember(null);
  }
  if (drag.kind === "point") {
    const p = { x: (clamp(sx, 6, g.sw - 6) - g.fx) / g.s, y: (clamp(sy, 6, g.sh - 6) - g.fy) / g.s };
    if (S.tool === "place") {
      const src = S.src.slice();
      src[drag.idx] = p;
      placeTo(src, drag.M0);
    } else {
      const dst = S.dst.slice();
      dst[drag.idx] = p;
      shiftTo(dst);
    }
  } else {
    const dx = (ev.clientX - drag.x0) / g.s;
    const dy = (ev.clientY - drag.y0) / g.s;
    const off = (q: Quad) => q.map((p) => ({ x: p.x + dx, y: p.y + dy }));
    if (S.tool === "place") placeTo(off(drag.src0), drag.M0);
    else shiftTo(off(drag.dst0));
  }
}

function dragEnd() {
  window.removeEventListener("pointermove", dragMove);
  window.removeEventListener("pointerup", dragEnd);
  window.removeEventListener("pointercancel", dragEnd);
  drag = null;
  set({ dragging: false });
  settle();
}
