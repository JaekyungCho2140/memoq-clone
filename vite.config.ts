/// <reference types="vitest" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig(async () => ({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  test: {
    environment: "node",
    globals: true,
    exclude: ["tests/smoke/**", "node_modules/**"],
    coverage: {
      provider: "v8",
      include: [
        "src/stores/**/*.ts",
        "src/adapters/**/*.ts",
        "src/hooks/**/*.ts",
        "src/components/TmPanel/**/*.tsx",
        "src/components/TbPanel/**/*.tsx",
      ],
      exclude: [
        "**/__tests__/**",
        "**/*.test.{ts,tsx}",
        // Platform-specific adapter implementations: these call platform APIs
        // (@tauri-apps/api, browser fetch/WebSocket) that require either the
        // native Tauri runtime or a live REST server. Verified via E2E/smoke tests.
        "src/tauri/**/*.ts",
        "src/adapters/tauri.ts",
        "src/adapters/web.ts",
      ],
      thresholds: {
        statements: 80,
        branches: 80,
        functions: 80,
        lines: 80,
      },
    },
  },
}));
