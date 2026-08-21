import { defineConfig } from "vitest/config";

// EP-035 M5: run only the real TypeScript test sources. Generated JS
// emit artifacts (pre-outDir build strays) must never be picked up as
// duplicate test files - two copies racing on the same bundle directory
// is a real failure mode (verify-chain discovery 2026-08-21).
export default defineConfig({
  test: {
    include: ["src/__tests__/**/*.test.ts"],
  },
});
