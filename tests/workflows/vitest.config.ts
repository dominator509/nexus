import { defineConfig } from "vitest/config";

/**
 * EP-006 M3 real-server integration suite.
 *
 * Tests spawn REAL ephemeral containers (postgres:18.4 + Temporal server
 * 1.31.2) and therefore need generous timeouts. One single fork shares a
 * module-level session cache (helpers/session.ts) so the whole suite uses
 * ONE server stack: one namespace and one server per run (EP-006
 * fallback doctrine), started once and torn down at suite end.
 *
 * Teardown is a HARD invariant: the primary path is explicit async
 * disposal from try/finally (stack.dispose() / session.shutdown() in the
 * final teardown test). globalTeardown below is the suite-level safety
 * net that disposes every registered stack even if the fork dies;
 * process-exit hooks are the last-resort emergency net only.
 */
export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    testTimeout: 120_000,
    hookTimeout: 120_000,
    pool: "forks",
    poolOptions: {
      forks: {
        singleFork: true,
      },
    },
    // Share the module registry across test files: session.ts caches the
    // ONE server stack + ONE SDK Runtime per process. With per-file
    // isolation the cached session resets between files while the
    // process-level Runtime singleton does not, so a second file would
    // call Runtime.install() again and throw.
    isolate: false,
    fileParallelism: false,
    sequence: { concurrent: false },
    // Suite-level teardown net: vitest 3 globalSetup pattern - the
    // `teardown` export in global-setup.ts disposes every registered
    // stack even when the fork dies mid-run (see global-setup.ts).
    globalSetup: "./global-setup.ts",
  },
});
