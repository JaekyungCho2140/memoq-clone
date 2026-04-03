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
    coverage: {
      provider: "v8",
      include: [
        "src/stores/**/*.ts",
        "src/tauri/**/*.ts",
        "src/components/TmPanel/**/*.tsx",
        "src/components/TbPanel/**/*.tsx",
      ],
      exclude: [
        "**/__tests__/**",
        "**/*.test.{ts,tsx}",
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
