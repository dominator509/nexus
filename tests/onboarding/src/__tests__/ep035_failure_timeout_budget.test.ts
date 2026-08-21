/**
 * EP-035 M4 forced failure: timeout and budget exhaustion.
 *
 * The REAL failure mechanism is a declared budget: a statement timeout
 * on the production transport. The store must map the exhaustion to
 * Timeout (SPEC-006), never retry blindly, and never leak the raw
 * driver message.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { ErrorCode } from "@nexus/setup";
import { OnboardingDb } from "@nexus/onboarding";
import { freshDb, startStack, stopStack, type TestStack } from "./harness.js";

describe("ep035_failure_timeout_budget", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  it("maps statement-timeout budget exhaustion to Timeout (SPEC-006)", async () => {
    const db = await freshDb(stack);
    // Exhaust a declared budget: 50ms statement timeout on a 2s sleep.
    const budget = new OnboardingDb({
      host: "127.0.0.1",
      port: stack.pgPort,
      user: "nexus",
      password: "nexus-test",
      database: "nexus",
      statementTimeoutMillis: 50,
    });
    try {
      await expect(budget.query("SELECT pg_sleep(2)")).rejects.toMatchObject({
        code: ErrorCode.Timeout,
      });
    } finally {
      await budget.close();
    }
  }, 30000);

  it("keeps the durable store usable after a timeout (bounded recovery)", async () => {
    const db = await freshDb(stack);
    const budget = new OnboardingDb({
      host: "127.0.0.1",
      port: stack.pgPort,
      user: "nexus",
      password: "nexus-test",
      database: "nexus",
      statementTimeoutMillis: 50,
    });
    try {
      await expect(budget.query("SELECT pg_sleep(2)")).rejects.toMatchObject({
        code: ErrorCode.Timeout,
      });
    } finally {
      await budget.close();
    }
    // The provider itself is healthy; the next normal statement succeeds.
    await expect(db.query("SELECT 1")).resolves.toBeDefined();
  }, 30000);
});
