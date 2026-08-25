import { defineConfig } from "vitest/config";

// EP-042 M5: run only the REAL offline-bundle proof suite (src/bundle).
// The M2 unit suite, M3 integration suite, and M4 failure suite keep
// their own configs untouched; the M5 gate runs this suite explicitly.
export default defineConfig({
  test: {
    include: ["src/bundle/**/*.test.ts"],
    testTimeout: 60_000,
    hookTimeout: 60_000,
  },
});
