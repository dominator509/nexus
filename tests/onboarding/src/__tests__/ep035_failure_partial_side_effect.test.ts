/**
 * EP-035 M4 forced failure: partial side effects and recovery.
 *
 * The REAL failure mechanism is an interrupted mutation: a provider
 * dies between dependent operations, and the durable recovery
 * checkpoint must classify the mutation UNKNOWN - never retry-safe,
 * never silently resumed. Reconciliation requires a real readback.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  ErrorCode,
  EnrollmentCredential,
  RecoveryEvidence,
  Spec006Error,
} from "@nexus/setup";
import {
  RecoveryCheckpointStore,
  EnrollmentTokenStore,
  OnboardingDb,
} from "@nexus/onboarding";
import {
  freshDb,
  killPostgres,
  startStack,
  stopStack,
  type TestStack,
} from "./harness.js";

describe("ep035_failure_partial_side_effect", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  it("refuses to persist a retry-safe decision for an unknown mutation", async () => {
    const db = await freshDb(stack);
    const store = new RecoveryCheckpointStore(db);

    // Evidence proves NOTHING about the mutation -> not retry-safe.
    const unknown = RecoveryEvidence.parse({
      failure_class: "AMBIGUOUS",
      mutation_known: false,
      correlation_id: "50000000-0000-0000-0000-000000000001",
    });
    const decision = RecoveryCheckpointStore.decide(unknown);
    expect(decision.retry_safe).toBe(false);

    await expect(
      store.record(
        "50000000-0000-0000-0000-000000000002",
        "partial-mutation-1",
        "owner_bootstrap",
        unknown,
        decision,
        1_700_000_000,
        "50000000-0000-0000-0000-000000000001",
      ),
    ).resolves.toMatchObject({
      mutation_state: "UNKNOWN",
      retry_safe: false,
    });

    // Durable CHECK also refuses a crafted retry-safe UNKNOWN row.
    await expect(
      db.query(
        `INSERT INTO onboarding_recovery_checkpoint
           (checkpoint_id, mutation_id, mutation_kind, mutation_state,
            failure_class, outcome, retry_safe, created_at_unix_s, detail,
            correlation_id)
         VALUES ($1, 'crafted-blind-retry', 'owner_bootstrap', 'UNKNOWN',
                 'TIMEOUT', 'RETRYABLE', TRUE, 100, 'blind', $2)`,
        [
          "50000000-0000-0000-0000-000000000003",
          "50000000-0000-0000-0000-000000000004",
        ],
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Validation });
  }, 30000);

  it("allows retry only after reconciliation with a real readback", async () => {
    const db = await freshDb(stack);
    const store = new RecoveryCheckpointStore(db);

    // A TIMEOUT where the readback proves no mutation occurred is
    // retry-safe (the durable store persists it as RECONCILED so the
    // UNKNOWN-never-retry-safe invariant holds).
    const evidence = RecoveryEvidence.parse({
      failure_class: "TIMEOUT",
      mutation_known: true,
      mutation_occurred: false, // readback proved no mutation
      correlation_id: "50000000-0000-0000-0000-000000000010",
    });
    const decision = RecoveryCheckpointStore.decide(evidence);
    expect(decision.retry_safe).toBe(true);

    const row = await store.record(
      "50000000-0000-0000-0000-000000000011",
      "partial-mutation-2",
      "owner_bootstrap",
      evidence,
      decision,
      1_700_000_000,
      "50000000-0000-0000-0000-000000000010",
    );
    // Retry-safe + known-no-mutation persists as RECONCILED (durable
    // invariant holds: UNKNOWN can never be retry-safe).
    expect(row.mutation_state).toBe("RECONCILED");
    expect(row.retry_safe).toBe(true);
  }, 30000);

  it("reports ambiguous recovery without fabricating a decision", async () => {
    const db = await freshDb(stack);
    const store = new RecoveryCheckpointStore(db);

    const evidence = RecoveryEvidence.parse({
      failure_class: "TIMEOUT",
      mutation_known: false,
      correlation_id: "50000000-0000-0000-0000-000000000030",
    });
    const decision = RecoveryCheckpointStore.decide(evidence);
    expect(decision.retry_safe).toBe(false);
    expect(decision.outcome).toBe("RECONCILE");

    const row = await store.record(
      "50000000-0000-0000-0000-000000000031",
      "timeout-mutation-1",
      "enrollment_claim",
      evidence,
      decision,
      1_700_000_000,
      "50000000-0000-0000-0000-000000000030",
    );
    expect(row.mutation_state).toBe("UNKNOWN");
    expect(row.retry_safe).toBe(false);
    // A later real readback reconciles the checkpoint.
    const reconciled = await store.reconcile(
      "timeout-mutation-1",
      "RECONCILED",
      1_700_000_010,
      "50000000-0000-0000-0000-000000000030",
    );
    expect(reconciled.mutation_state).toBe("RECONCILED");
  }, 30000);

  // NOTE: this test destroys the shared postgres container; it MUST be
  // the last test in this file so later tests are not starved.
  it("fails closed when the provider dies mid-mutation (no partial success)", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);

    // First durable write succeeds.
    await store.issue(
      EnrollmentCredential.parse({
        credential_id: "50000000-0000-0000-0000-000000000020",
        kind: "BOOTSTRAP_TOKEN",
        issued_at_unix_s: 1_700_000_000,
        expires_at_unix_s: 1_700_100_000,
        state: "ISSUED",
        secret: "nexus-secret-partialabcdefghijklmnopqrstuvwxyz",
        nonce: "nexus-nonce-partial0123456789abcdef0123456789",
      }),
    );

    // Provider terminates before the dependent readback.
    killPostgres(stack);

    await expect(
      store.read("50000000-0000-0000-0000-000000000020"),
    ).rejects.toMatchObject({ code: ErrorCode.Unavailable });
  }, 60000);
});
