/**
 * EP-035 M4 forced failure: unavailable dependency.
 *
 * The REAL failure mechanism is provider termination: the postgres or
 * NATS container is removed mid-suite and the production store must map
 * the loss to the canonical SPEC-006 Unavailable class - never a
 * fabricated success and never a raw driver exception leaking out.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { ErrorCode } from "@nexus/setup";
import { OwnerBootstrapStore, derivePrincipalId } from "@nexus/onboarding";
import { OwnerBootstrapRequest } from "@nexus/setup";
import {
  freshDb,
  killPostgres,
  startStack,
  stopStack,
  type TestStack,
} from "./harness.js";

describe("ep035_failure_unavailable_dependency", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  it("maps postgres container termination to Unavailable (SPEC-006)", async () => {
    const db = await freshDb(stack);
    const store = new OwnerBootstrapStore(db);

    // Prove the provider was actually up first.
    const req1 = OwnerBootstrapRequest.parse({
      idempotency_key: "failure-owner-1",
      owner_name: "Failure Owner",
      owner_email: "owner@nexus.test",
      correlation_id: "10000000-0000-0000-0000-000000000001",
    });
    await expect(
      store.initialize(req1, derivePrincipalId(req1), 1_700_000_000),
    ).resolves.toMatchObject({ kind: "INITIALIZED" });

    // Terminate the real provider.
    killPostgres(stack);

    // The production transport must fail closed with the canonical class.
    await expect(
      store.readOwner("10000000-0000-0000-0000-000000000001"),
    ).rejects.toMatchObject({ code: ErrorCode.Unavailable });
  }, 60000);

  it("cannot initialize an owner after the provider died (no fabrication)", async () => {
    // The provider was terminated by the previous test; the durable
    // boundary is gone. A freshDb/migrate would reject before the
    // store call, so exercise the surviving handle directly.
    const store = new OwnerBootstrapStore(stack.db);
    killPostgres(stack);

    const req2 = OwnerBootstrapRequest.parse({
      idempotency_key: "failure-owner-2",
      owner_name: "Failure Owner Two",
      owner_email: "owner2@nexus.test",
      correlation_id: "10000000-0000-0000-0000-000000000002",
    });
    await expect(
      store.initialize(req2, derivePrincipalId(req2), 1_700_000_000),
    ).rejects.toMatchObject({ code: ErrorCode.Unavailable });
  }, 60000);
});
