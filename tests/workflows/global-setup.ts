/**
 * EP-006 M3 suite-level teardown net (vitest 3 globalSetup pattern).
 *
 * vitest 3 removed `globalTeardown`; a `globalSetup` file may export a
 * `teardown` function which vitest runs after the whole suite (in the
 * main process, even when tests failed - see Vitest.close() ->
 * _teardownGlobalSetup()).
 *
 * The fork process registers every started stack in the registry file
 * (tests/workflows/src/helpers/stack.ts); this teardown disposes every
 * registered stack with REAL docker, accumulating failures. Any failure
 * here throws and fails the vitest run - teardown failures are test
 * failures.
 *
 * This is a safety net AND the shared session's primary disposer: the
 * shared session (started lazily by the first test file, in vitest's
 * single fork) cannot be disposed from any individual test file without
 * depending on file order (vitest does NOT guarantee alphabetical
 * order), so the suite-level teardown owns it. The per-stack primary
 * pattern (`stack = await startStack(); try {...} finally { await
 * stack.dispose(); }`) is proven end-to-end by the teardown regression
 * test. The process-exit hook in helpers/session.ts is the last-resort
 * emergency net only.
 */

import { readFileSync, unlinkSync } from "node:fs";

import {
  disposeStackResources,
  STACK_STATE_FILE,
  type StackResources,
} from "./src/helpers/stack.js";

export async function setup(): Promise<void> {
  // No suite-level setup needed: stacks are started lazily by tests.
}

export async function teardown(): Promise<void> {
  let raw: string;
  try {
    raw = readFileSync(STACK_STATE_FILE, "utf8");
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message.includes("ENOENT")) {
      // Nothing registered: nothing to clean.
      return;
    }
    throw new Error(
      `EP-006 globalTeardown: cannot read stack registry ${STACK_STATE_FILE}: ${message}`,
    );
  }

  let entries: StackResources[];
  try {
    const parsed = JSON.parse(raw) as { entries?: StackResources[] };
    entries = Array.isArray(parsed.entries) ? parsed.entries : [];
  } catch (error) {
    throw new Error(
      `EP-006 globalTeardown: stack registry ${STACK_STATE_FILE} is corrupt: ${String(error)}`,
    );
  }

  const failures: string[] = [];
  for (const entry of entries) {
    try {
      await disposeStackResources(entry);
    } catch (error) {
      failures.push(String(error));
    }
  }

  if (failures.length === 0) {
    try {
      unlinkSync(STACK_STATE_FILE);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (!message.includes("ENOENT")) {
        failures.push(`remove registry file ${STACK_STATE_FILE}: ${message}`);
      }
    }
  }

  if (failures.length > 0) {
    throw new Error(
      `EP-006 globalTeardown: ${failures.length} stack disposal failure(s):\n${failures
        .map((failure) => `- ${failure}`)
        .join("\n")}`,
    );
  }
}
