import { listen } from "@tauri-apps/api/event";

/** Runs when a native menu item is chosen. The item's id names the command. */
export const onMenuCommand = (handler: (id: string) => void) =>
  listen<string>("menu-command", (e) => handler(e.payload));
