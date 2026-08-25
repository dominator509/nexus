import { defineConfig } from "vitest/config";

// EP-042 M4: run only the REAL failure/abuse/observability proof suite
// (src/failure). The M2 unit suite and M3 integration suite keep their
// own configs untouched; the M4 gate runs this suite explicitly.
export default defineConfig({
  test: {
    include: ["src/failure/**/*.test.ts"],
    testTimeout: 60_000,
    hookTimeout: 60_000,
  },
});
