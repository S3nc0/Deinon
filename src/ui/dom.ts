export const $ = <T extends HTMLElement = HTMLElement>(id: string): T => {
  const el = document.getElementById(id);
  if (!el) throw new Error(`Missing element #${id}`);
  return el as T;
};

export const $svg = (id: string): SVGElement => document.getElementById(id) as unknown as SVGElement;

export const $input = (id: string): HTMLInputElement => $<HTMLInputElement>(id);

export const clamp = (v: number, a: number, b: number) => Math.min(b, Math.max(a, v));

export const r2 = (n: number) => Math.round(n * 100) / 100;

export function place(el: HTMLElement, x: number, y: number, w?: number | null, h?: number | null) {
  el.style.left = r2(x) + "px";
  el.style.top = r2(y) + "px";
  if (w != null) el.style.width = r2(w) + "px";
  if (h != null) el.style.height = r2(h) + "px";
}

export function setPressed(el: Element, on: boolean) {
  el.setAttribute("aria-pressed", on ? "true" : "false");
}
