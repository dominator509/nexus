/**
 * EP-035 M3 integration: real deployment intent persistence.
 *
 * A selected deployment profile is INTENT ONLY. The durable store keeps
 * SELECTED distinct from VERIFIED; verification requires an evidence
 * record. A probe/readback never mutates the user's selected intent.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { randomUUID } from "node:crypto";
import {
  DeploymentSelectionRequest,
  DeploymentVerificationRequest,
  ErrorCode,
} from "@nexus/setup";
import {
  DeploymentIntentStore,
  verifyDeploymentIntentRecord,
} from "../../stores/deployment-intent.store.js";
import { freshDb, startStack, stopStack, type TestStack } from "./harness.js";

describe("ep035_integration_deployment_intent", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  function selection(): DeploymentSelectionRequest {
    return DeploymentSelectionRequest.parse({
      profile: {
        id: "profile-local",
        mode: "FULLY_LOCAL",
        release_channel: "STABLE",
        components: ["core", "edge"],
        nodes: [{ id: "home-node", role: "edge" }],
        backup: { enabled: true },
        remote_access: { enabled: false },
      },
      correlation_id: randomUUID(),
    });
  }

  it("persists a selection as intent-only (SELECTED, never VERIFIED)", async () => {
    const db = await freshDb(stack);
    const store = new DeploymentIntentStore(db);
    const intentId = randomUUID();
    const req = selection();

    const row = await store.recordSelection(intentId, req, 1_700_000_000);
    expect(row.verification_state).toBe("SELECTED");
    expect(row.verified_at_unix_s).toBeNull();

    // Exact-target readback through the contract value object.
    const record = verifyDeploymentIntentRecord(
      await store.read(intentId).then((r) => r!),
    );
    expect(record.profile.mode).toBe("FULLY_LOCAL");
    expect(record.verification.state).toBe("UNVERIFIED");

    await db.close();
  });

  it("requires evidence to become VERIFIED", async () => {
    const db = await freshDb(stack);
    const store = new DeploymentIntentStore(db);
    const intentId = randomUUID();
    const req = selection();
    await store.recordSelection(intentId, req, 1_700_000_000);

    // Verification without evidence is rejected by the contract before
    // reaching the store (canonical Verification error).
    let verificationError: unknown;
    try {
      DeploymentVerificationRequest.parse({
        correlation_id: randomUUID(),
        state: "VERIFIED",
      });
    } catch (err) {
      verificationError = err;
    }
    expect(verificationError).toMatchObject({ code: ErrorCode.Verification });

    // With evidence the durable row becomes VERIFIED.
    const vreq = DeploymentVerificationRequest.parse({
      correlation_id: randomUUID(),
      state: "VERIFIED",
      evidence: {
        verified_at_unix_s: 1_700_000_050,
        evidence_id: randomUUID(),
        verifier: "local-probe",
      },
    });
    const row = await store.recordVerification(intentId, vreq, 1_700_000_050);
    expect(row.verification_state).toBe("VERIFIED");

    const record = verifyDeploymentIntentRecord(
      await store.read(intentId).then((r) => r!),
    );
    expect(record.verification.state).toBe("VERIFIED");
    expect(record.verification.evidence?.verifier).toBe("local-probe");

    await db.close();
  });

  it("a probe readback does not mutate the selected intent", async () => {
    const db = await freshDb(stack);
    const store = new DeploymentIntentStore(db);
    const intentId = randomUUID();
    const req = selection();
    await store.recordSelection(intentId, req, 1_700_000_000);

    // Simulate a probe: read twice, never write.
    await store.read(intentId);
    const row = await store.read(intentId);
    expect(row!.verification_state).toBe("SELECTED");
    expect(row!.mode).toBe("FULLY_LOCAL");

    await db.close();
  });

  it("rejects verification on a missing or already-verified intent (Conflict)", async () => {
    const db = await freshDb(stack);
    const store = new DeploymentIntentStore(db);
    const intentId = randomUUID();
    const req = selection();
    await store.recordSelection(intentId, req, 1_700_000_000);

    const vreq = DeploymentVerificationRequest.parse({
      correlation_id: randomUUID(),
      state: "VERIFIED",
      evidence: {
        verified_at_unix_s: 1_700_000_050,
        evidence_id: randomUUID(),
        verifier: "local-probe",
      },
    });
    await store.recordVerification(intentId, vreq, 1_700_000_050);

    // Second verification -> Conflict (already VERIFIED).
    await expect(
      store.recordVerification(intentId, vreq, 1_700_000_060),
    ).rejects.toMatchObject({ code: ErrorCode.Conflict });

    // Verification of a nonexistent intent -> Conflict.
    await expect(
      store.recordVerification(randomUUID(), vreq, 1_700_000_060),
    ).rejects.toMatchObject({ code: ErrorCode.Conflict });

    await db.close();
  });
});
