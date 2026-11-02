import { S, set, type Snapshot } from "./state.ts";

const LIMIT = 50;
const GROUP_MS = 700;

const snapshot = (): Snapshot => ({
  src: S.src,
  dst: S.dst,
  goodSrc: S.goodSrc,
  good: S.good,
  base: S.base,
  camBase: S.camBase,
  cam: S.cam,
});

let lastGroup: string | null = null;
let lastTime = 0;

/** Call before a change. Repeated changes in one group within 700 ms share one undo step. */
export function remember(group: string | null) {
  const now = Date.now();
  if (group && group === lastGroup && now - lastTime < GROUP_MS) {
    lastTime = now;
    return;
  }
  lastGroup = group;
  lastTime = now;
  set({ past: S.past.slice(-(LIMIT - 1)).concat([snapshot()]), future: [] });
}

export function undo() {
  if (!S.past.length) return;
  lastGroup = null;
  set({
    ...S.past[S.past.length - 1],
    invalid: false,
    past: S.past.slice(0, -1),
    future: S.future.concat([snapshot()]),
  });
}

export function redo() {
  if (!S.future.length) return;
  lastGroup = null;
  set({
    ...S.future[S.future.length - 1],
    invalid: false,
    future: S.future.slice(0, -1),
    past: S.past.concat([snapshot()]),
  });
}
