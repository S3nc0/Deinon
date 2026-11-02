import { $ } from "./dom.ts";

/** Shows an error that says what to do, until it is dismissed or the next action starts. */
export function showMessage(text: string) {
  $("appMessageText").textContent = text;
  $("appMessage").hidden = false;
}

export function clearMessage() {
  $("appMessage").hidden = true;
}

export function showLoading(text: string | null) {
  $("appLoading").hidden = text === null;
  $("appLoadingText").textContent = text ?? "";
}
