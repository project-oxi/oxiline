import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Pure-logic tests only (node env — no DOM/jsdom). Pointer interactions are
// verified via the audit/ browser harness, NOT here.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
