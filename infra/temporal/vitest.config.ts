import { defineConfig } from "vitest/config";

/**
 * @nexus/temporal unit suite (vitest).
 *
 * The AUD-023 interceptor unit tests (ep006_unit_failure.test.ts) prove
 * the try/catch logic with a labeled `next()` test double - that is a
 * UNIT test and stays. The REAL boundary proof lives in
 * ep006_official_environment_interceptor.test.ts: TestWorkflowEnvironment
 * (the official @temporalio/testing environment) launches a REAL Temporal
 * server binary; the real worker + real NexusFailureInterceptor + a real
 * activity throwing NexusWorkflowError cross a genuine gRPC boundary.
 * TESTING.md line 36: "Temporal tests include the official test
 * environment and at least one real server E2E."
 */
export default defineConfig({
  test: {
    include: ["src/__tests__/**/*.test.ts"],
    testTimeout: 180_000,
    hookTimeout: 180_000,
    // TestWorkflowEnvironment installs its own SDK Runtime singleton; the
    // interceptor suite must run before/after unit tests without leaking
    // workers. File parallelism off keeps worker slot management honest.
    fileParallelism: false,
  },
});
