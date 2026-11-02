import { $, $input } from "./dom.ts";
import { startDrag } from "./drag.ts";
import { activePts, setPoint, settle } from "./edits.ts";
import { remember } from "./history.ts";
import { set } from "./state.ts";

const NAMES = ["Top left", "Top right", "Bottom right", "Bottom left"];

export const pointEls: HTMLButtonElement[] = [];
export const ghostEls: HTMLDivElement[] = [];
export const cardEls: HTMLDivElement[] = [];

/** Builds the 4 points, their start markers and their cards. `rerender` redraws at once. */
export function buildPoints(rerender: () => void) {
  for (let i = 0; i < 4; i++) {
    const el = document.createElement("button");
    el.type = "button";
    el.className = "point";
    el.dataset.corner = String(i);
    el.innerHTML =
      '<span class="point__dot" aria-hidden="true"></span><span class="point__tag" aria-hidden="true">' + (i + 1) + "</span>";
    el.addEventListener("pointerdown", (e) => startDrag(e, "point", i));
    el.addEventListener("focus", () => set({ selected: i }));
    el.addEventListener("keydown", (e) => {
      const dir = ({ ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] } as Record<string, number[]>)[e.key];
      if (!dir) return;
      e.preventDefault();
      const step = e.shiftKey ? 10 : 1;
      const p = activePts()[i];
      remember("key-" + i);
      setPoint(i, { x: p.x + dir[0] * step, y: p.y + dir[1] * step });
    });
    $("points").appendChild(el);
    pointEls[i] = el;

    const ghost = document.createElement("div");
    ghost.className = "ghost";
    $("ghosts").appendChild(ghost);
    ghostEls[i] = ghost;
  }

  // Cards sit in the same arrangement as the corners: 1 2 on top, 4 3 below.
  for (const i of [0, 1, 3, 2]) {
    const card = document.createElement("div");
    card.className = "point-card";
    card.innerHTML =
      '<div class="point-card__head"><span class="point-card__n" aria-hidden="true">' + (i + 1) + "</span><span>" + NAMES[i] + "</span></div>" +
      '<div class="point-card__xy">' +
      '<div class="field field--sm"><label class="field__label" for="p' + i + 'x">X</label><input class="field__input" id="p' + i + 'x" type="text" inputmode="decimal" autocomplete="off"></div>' +
      '<div class="field field--sm"><label class="field__label" for="p' + i + 'y">Y</label><input class="field__input" id="p' + i + 'y" type="text" inputmode="decimal" autocomplete="off"></div>' +
      "</div>";
    $("cards").appendChild(card);
    cardEls[i] = card;

    for (const axis of ["x", "y"] as const) {
      const input = $input("p" + i + axis);
      input.addEventListener("focus", () => set({ selected: i }));
      input.addEventListener("input", () => {
        const n = Number(input.value);
        if (input.value.trim() === "" || !Number.isFinite(n)) return;
        remember("field-" + i + axis);
        setPoint(i, { ...activePts()[i], [axis]: n });
      });
      input.addEventListener("blur", () => {
        settle();
        rerender();
      });
      input.addEventListener("keydown", (e) => {
        if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;
        e.preventDefault();
        const step = (e.shiftKey ? 10 : 1) * (e.key === "ArrowUp" ? 1 : -1);
        const p = activePts()[i];
        remember("field-" + i + axis);
        setPoint(i, { ...p, [axis]: p[axis] + step });
        input.value = String(Math.round(activePts()[i][axis]) || 0);
      });
    }
  }
}
