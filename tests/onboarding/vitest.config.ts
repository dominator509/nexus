import { defineConfig } from "vitest/config";

// EP-035 M5 verify-chain correction: run only the real TypeScript test
// sources. Generated JS emit artifacts (pre-outDir build strays) must
// never be picked up as duplicate test files.
export default defineConfig({
  test: {
    include: ["src/__tests__/**/*.test.ts"],
  },
});
