/**
 * EP-035 M4 forced failure: duplicate request and concurrent abuse.
 *
 * The REAL failure mechanism is the durable uniqueness boundary:
 * concurrent double-claim of a one-time enrollment token, duplicate
 * first-owner bootstrap, and duplicate deployment verification. Exactly
 * one request wins; every replay is denied. No application mutex is
 * involved - the database row lock and unique index are the authority.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  DeploymentSelectionRequest,
  DeploymentVerificationRequest,
  EnrollmentCredential,
  ErrorCode,
  OwnerBootstrapRequest,
} from "@nexus/setup";
import {
  DeploymentIntentStore,
  EnrollmentTokenStore,
  OwnerBootstrapStore,
  derivePrincipalId,
} from "@nexus/onboarding";
import { freshDb, startStack, stopStack, type TestStack } from "./harness.js";

function freshCredential(): EnrollmentCredential {
  return EnrollmentCredential.parse({
    credential_id: "30000000-0000-0000-0000-000000000001",
    kind: "BOOTSTRAP_TOKEN",
    issued_at_unix_s: 1_700_000_000,
    expires_at_unix_s: 1_700_100_000,
    state: "ISSUED",
    secret: "nexus-secret-abcdefghijklmnopqrstuvwxyz012345",
    nonce: "nexus-nonce-0123456789abcdef0123456789abcdef",
  });
}

const PROFILE = {
  id: "profile-failure-dup",
  mode: "MANAGED",
  release_channel: "STABLE",
  components: ["core"],
  nodes: [],
  backup: {},
  remote_access: {},
};

describe("ep035_failure_duplicate_request", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  it("claims a one-time token exactly once under concurrency", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);
    const credential = freshCredential();
    await store.issue(credential);

    // Two concurrent claims race the row lock; exactly one wins.
    const [a, b] = await Promise.all([
      store.claim(
        credential.credential_id,
        1_700_000_050,
        "30000000-0000-0000-0000-000000000003",
      ),
      store.claim(
        credential.credential_id,
        1_700_000_050,
        "30000000-0000-0000-0000-000000000004",
      ),
    ]);
    expect(a === b).toBe(false);

    // Replay after the claim is denied.
    await expect(
      store.claim(
        credential.credential_id,
        1_700_000_060,
        "30000000-0000-0000-0000-000000000005",
      ),
    ).resolves.toBe(false);
  }, 30000);

  it("rejects a competing first owner with Conflict (durable index)", async () => {
    const db = await freshDb(stack);
    const store = new OwnerBootstrapStore(db);

    const req = (key: string, email: string, corr: string) =>
      OwnerBootstrapRequest.parse({
        idempotency_key: key,
        owner_name: "Duplicate Owner",
        owner_email: email,
        correlation_id: corr,
      });

    const first = req(
      "dup-owner-alpha",
      "alpha@nexus.test",
      "30000000-0000-0000-0000-000000000010",
    );
    await expect(
      store.initialize(first, derivePrincipalId(first), 1_700_000_000),
    ).resolves.toMatchObject({ kind: "INITIALIZED" });

    // Same logical replay -> ALREADY_INITIALIZED (idempotent).
    await expect(
      store.initialize(first, derivePrincipalId(first), 1_700_000_010),
    ).resolves.toMatchObject({ kind: "ALREADY_INITIALIZED" });

    // Competing first owner -> CONFLICT (unique partial index).
    const second = req(
      "dup-owner-beta",
      "beta@nexus.test",
      "30000000-0000-0000-0000-000000000011",
    );
    await expect(
      store.initialize(second, derivePrincipalId(second), 1_700_000_020),
    ).resolves.toMatchObject({ kind: "CONFLICT" });
  }, 30000);

  it("refuses a second verification of the same deployment intent", async () => {
    const db = await freshDb(stack);
    const store = new DeploymentIntentStore(db);
    const intentId = "30000000-0000-0000-0000-000000000020";
    const selection = DeploymentSelectionRequest.parse({
      profile: PROFILE,
      correlation_id: "30000000-0000-0000-0000-000000000021",
    });
    await store.recordSelection(
      intentId,
      selection,
      1_700_000_000,
      "30000000-0000-0000-0000-000000000021",
    );

    const verify = (corr: string) =>
      DeploymentVerificationRequest.parse({
        correlation_id: corr,
        state: "VERIFIED",
        evidence: {
          verified_at_unix_s: 1_700_000_010,
          evidence_id: "evidence-dup-1",
          verifier: "host-probe",
        },
      });

    await expect(
      store.recordVerification(
        intentId,
        verify("30000000-0000-0000-0000-000000000022"),
        1_700_000_010,
        "30000000-0000-0000-0000-000000000022",
      ),
    ).resolves.toMatchObject({ verification_state: "VERIFIED" });

    // Second verification on an already-verified intent -> Conflict.
    await expect(
      store.recordVerification(
        intentId,
        verify("30000000-0000-0000-0000-000000000023"),
        1_700_000_020,
        "30000000-0000-0000-0000-000000000023",
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Conflict });
  }, 30000);
});
