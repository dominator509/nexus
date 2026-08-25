import { defineConfig } from "vitest/config";

// EP-042 M3: run only the REAL integration test sources against a live
// ephemeral S3-compatible container. The M2 gate keeps running the
// deterministic unit suite (src/__tests__) untouched; the M3 gate runs
// this suite explicitly with NEXUS_RELEASE_* env supplied by
// scripts/ep042-m3-tests.sh.
export default defineConfig({
  test: {
    include: ["src/integration/**/*.test.ts"],
    testTimeout: 60_000,
    hookTimeout: 60_000,
  },
});
