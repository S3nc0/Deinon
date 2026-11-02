import { defineConfig } from "vitest/config";
// @ts-expect-error type error without @types/node package
import process from "node:process";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  clearScreen: false,
  build: {
    // Keep fonts as files: the content security policy does not allow data: URLs.
    assetsInlineLimit: 0,
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
});
