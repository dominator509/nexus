/**
 * EP-035 M3 integration: real RecoveryFlow no-blind-replay boundary.
 *
 * An ambiguous external mutation is never retried blindly. The durable
 * checkpoint stores UNKNOWN -> retry_safe FALSE; only a reconciliation
 * readback may flip the decision to retry-safe. The SQL CHECK enforces
 * this invariant even if a buggy caller tries to persist a bad decision.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { randomUUID } from "node:crypto";
import {
  ErrorCode,
  type RecoveryEvidence,
  RecoveryDecision,
  RecoveryEvidence as RecoveryEvidenceClass,
} from "@nexus/setup";
import { RecoveryCheckpointStore } from "../../stores/recovery-checkpoint.store.js";
import { freshDb, startStack, stopStack, type TestStack } from "./harness.js";

/** Build a canonical RecoveryEvidence value object. */
function evidence(
  failureClass: RecoveryEvidence["failure_class"],
  overrides: Partial<Omit<RecoveryEvidence, "failure_class">> = {},
): RecoveryEvidence {
  return RecoveryEvidenceClass.parse({
    failure_class: failureClass,
    mutation_known: overrides.mutation_known ?? false,
    ...(overrides.mutation_occurred === undefined
      ? {}
      : { mutation_occurred: overrides.mutation_occurred }),
    ...(overrides.mutation_state === undefined
      ? {}
      : { mutation_state: overrides.mutation_state }),
    ...(overrides.correlation_id === undefined
      ? {}
      : { correlation_id: overrides.correlation_id }),
  });
}

describe("ep035_integration_recovery_flow", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  it("persists RECONCILE for an ambiguous mutation and never retry-safe", async () => {
    const db = await freshDb(stack);
    const store = new RecoveryCheckpointStore(db);
    const mutationId = `mut-${randomUUID().slice(0, 8)}`;
    const now = 1_700_000_000;

    const decision = RecoveryCheckpointStore.decide(evidence("AMBIGUOUS"));
    expect(decision.outcome).toBe("RECONCILE");
    expect(decision.retry_safe).toBe(false);

    const row = await store.record(
      randomUUID(),
      mutationId,
      "owner.create",
      evidence("AMBIGUOUS"),
      decision,
      now,
    );
    expect(row.outcome).toBe("RECONCILE");
    expect(row.retry_safe).toBe(false);
    expect(row.mutation_state).toBe("UNKNOWN");

    await db.close();
  });

  it("a timeout with unknown mutation outcome is never blind-retried", async () => {
    const db = await freshDb(stack);
    const store = new RecoveryCheckpointStore(db);
    const mutationId = `mut-${randomUUID().slice(0, 8)}`;
    const now = 1_700_000_000;

    const decision = RecoveryCheckpointStore.decide(evidence("TIMEOUT"));
    expect(decision.outcome).toBe("RECONCILE");
    expect(decision.retry_safe).toBe(false);

    await store.record(
      randomUUID(),
      mutationId,
      "edge.enroll",
      evidence("TIMEOUT"),
      decision,
      now,
    );
    const row = await store.read(mutationId);
    expect(row!.retry_safe).toBe(false);

    await db.close();
  });

  it("a timeout with known no-mutation is retry-safe", async () => {
    const db = await freshDb(stack);
    const store = new RecoveryCheckpointStore(db);
    const mutationId = `mut-${randomUUID().slice(0, 8)}`;
    const now = 1_700_000_000;

    const decision = RecoveryCheckpointStore.decide(
      evidence("TIMEOUT", { mutation_known: true, mutation_occurred: false }),
    );
    expect(decision.outcome).toBe("RETRYABLE");
    expect(decision.retry_safe).toBe(true);

    const row = await store.record(
      randomUUID(),
      mutationId,
      "edge.enroll",
      evidence("TIMEOUT", { mutation_known: true, mutation_occurred: false }),
      decision,
      now,
    );
    expect(row.retry_safe).toBe(true);

    await db.close();
  });

  it("reconciliation readback flips an ambiguous checkpoint to retry-safe", async () => {
    const db = await freshDb(stack);
    const store = new RecoveryCheckpointStore(db);
    const mutationId = `mut-${randomUUID().slice(0, 8)}`;
    const now = 1_700_000_000;

    const decision = RecoveryCheckpointStore.decide(evidence("AMBIGUOUS"));
    await store.record(
      randomUUID(),
      mutationId,
      "deployment.mutate",
      evidence("AMBIGUOUS"),
      decision,
      now,
    );

    // Reconcile: read provider state, confirm no mutation occurred.
    const reconciled = await store.reconcile(mutationId, "RECONCILED", now + 5);
    expect(reconciled.mutation_state).toBe("RECONCILED");
    expect(reconciled.reconciled_at_unix_s).toBe(now + 5);

    // After reconciliation WITH an explicit negative observation, retry
    // is safe (the readback proved no mutation occurred).
    const postDecision = RecoveryCheckpointStore.decide(
      evidence("AMBIGUOUS", {
        mutation_state: "RECONCILED",
        mutation_known: true,
        mutation_occurred: false,
      }),
    );
    expect(postDecision.outcome).toBe("RETRYABLE");
    expect(postDecision.retry_safe).toBe(true);

    await db.close();
  });

  it("AMBIGUOUS + RECONCILED without negative observation is NOT retry-safe (AUD-045)", async () => {
    const db = await freshDb(stack);
    const store = new RecoveryCheckpointStore(db);
    const mutationId = `mut-${randomUUID().slice(0, 8)}`;
    const now = 1_700_000_000;

    // Hostile: mutation_state RECONCILED alone (no mutation_known, no
    // explicit negative observation). This is exactly the finding:
    // recovery previously treated AMBIGUOUS + RECONCILED as safe to
    // retry, enabling duplicate consequential effects.
    const decision = RecoveryCheckpointStore.decide(
      evidence("AMBIGUOUS", { mutation_state: "RECONCILED" }),
    );
    expect(decision.outcome).toBe("RECONCILE");
    expect(decision.retry_safe).toBe(false);

    const row = await store.record(
      randomUUID(),
      mutationId,
      "deployment.mutate",
      evidence("AMBIGUOUS", { mutation_state: "RECONCILED" }),
      decision,
      now,
    );
    expect(row.retry_safe).toBe(false);
    expect(row.mutation_state).toBe("UNKNOWN");

    await db.close();
  });

  it("AMBIGUOUS + RECONCILED + mutation_known but NO occurred=false is NOT retry-safe (AUD-045)", async () => {
    const db = await freshDb(stack);
    const store = new RecoveryCheckpointStore(db);
    const mutationId = `mut-${randomUUID().slice(0, 8)}`;
    const now = 1_700_000_000;

    // Hostile: mutation is KNOWN and state RECONCILED, but the outcome
    // is NOT proven absent (mutation_occurred undefined/true). Without
    // the explicit negative observation retry is still unsafe.
    const decision = RecoveryCheckpointStore.decide(
      evidence("AMBIGUOUS", {
        mutation_state: "RECONCILED",
        mutation_known: true,
      }),
    );
    expect(decision.outcome).toBe("RECONCILE");
    expect(decision.retry_safe).toBe(false);

    const row = await store.record(
      randomUUID(),
      mutationId,
      "deployment.mutate",
      evidence("AMBIGUOUS", {
        mutation_state: "RECONCILED",
        mutation_known: true,
      }),
      decision,
      now,
    );
    expect(row.retry_safe).toBe(false);

    await db.close();
  });

  it("rejects persisting a retry-safe decision for an unknown mutation (durable invariant)", async () => {
    const db = await freshDb(stack);
    const store = new RecoveryCheckpointStore(db);
    const now = 1_700_000_000;

    // The store refuses at the application layer...
    await expect(
      store.record(
        randomUUID(),
        `mut-${randomUUID().slice(0, 8)}`,
        "owner.create",
        evidence("AMBIGUOUS"),
        new RecoveryDecision("RETRYABLE", "UNKNOWN", true, "bad decision"),
        now,
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Policy });

    // ...and the SQL CHECK would reject it even if bypassed.
    await expect(
      db.query(
        `INSERT INTO onboarding_recovery_checkpoint
           (checkpoint_id, mutation_id, mutation_kind, mutation_state,
            failure_class, outcome, retry_safe, created_at_unix_s, detail,
            correlation_id)
         VALUES ($1, $2, 'owner.create', 'UNKNOWN', 'AMBIGUOUS',
                 'RETRYABLE', TRUE, $3, 'bad decision', $4)`,
        [
          randomUUID(),
          `mut-sql-${randomUUID().slice(0, 8)}`,
          now,
          randomUUID(),
        ],
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Validation });

    await db.close();
  });

  it("classifies VALIDATION as non-retryable and CONFLICT as resume-checkpoint", async () => {
    const db = await freshDb(stack);
    const store = new RecoveryCheckpointStore(db);
    const now = 1_700_000_000;

    const validation = RecoveryCheckpointStore.decide(evidence("VALIDATION"));
    expect(validation.outcome).toBe("NON_RETRYABLE");
    expect(validation.retry_safe).toBe(false);

    const conflict = RecoveryCheckpointStore.decide(evidence("CONFLICT"));
    expect(conflict.outcome).toBe("RESUME_CHECKPOINT");
    expect(conflict.retry_safe).toBe(false);

    const auth = RecoveryCheckpointStore.decide(evidence("AUTHORIZATION"));
    expect(auth.outcome).toBe("REAUTHENTICATE");

    const internal = RecoveryCheckpointStore.decide(evidence("INTERNAL"));
    expect(internal.outcome).toBe("MANUAL_INTERVENTION");

    await db.close();
  });
});
