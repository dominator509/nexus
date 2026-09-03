/**
 * EP-035 M4 forced failure: denied permission and policy refusal.
 *
 * The REAL failure mechanism is the durable lifecycle boundary: a
 * revoked token, an expired token, a used token replay, and a policy
 * denial on an invalid integration ladder leap. The production stores
 * must deny every non-authorized transition and never coerce a state.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  EnrollmentCredential,
  ErrorCode,
  IntegrationCardRequest,
} from "@nexus/setup";
import { EnrollmentTokenStore, IntegrationStateStore } from "@nexus/onboarding";
import { freshDb, startStack, stopStack, type TestStack } from "./harness.js";

function credential(
  id: string,
  issued: number,
  expires: number,
): EnrollmentCredential {
  return EnrollmentCredential.parse({
    credential_id: id,
    kind: "BOOTSTRAP_TOKEN",
    issued_at_unix_s: issued,
    expires_at_unix_s: expires,
    state: "ISSUED",
    secret: secretFor(id),
    nonce: `nexus-nonce-${id.replaceAll("-", "")}0123456789abcdef0123456789`,
  });
}

/** Deterministic secret matching credential() so tests can prove it. */
function secretFor(id: string): string {
  return `nexus-secret-${id.replaceAll("-", "")}abcdefghijklmnopqrstuvwxyz`;
}

describe("ep035_failure_denied_permission", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  it("denies a revoked token permanently", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);
    const c = credential(
      "40000000-0000-0000-0000-000000000001",
      1_700_000_000,
      1_700_100_000,
    );
    await store.issue(c);

    await expect(store.revoke(c.credential_id, 1_700_000_010)).resolves.toBe(
      true,
    );

    // Revoked token can never be claimed again.
    await expect(
      store.claim(c.credential_id, secretFor(c.credential_id), 1_700_000_020),
    ).resolves.toBe(false);
    // Secret verification still answers (hash comparison) but the token
    // is not usable: claim is the durable gate.
    await expect(store.verifySecret(c.credential_id, c.secret)).resolves.toBe(
      true,
    );
  }, 30000);

  it("denies an expired token", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);
    const c = credential(
      "40000000-0000-0000-0000-000000000002",
      1_700_000_000,
      1_700_100_000,
    );
    await store.issue(c);

    // Claim after expiry is denied by the window condition.
    await expect(
      store.claim(c.credential_id, secretFor(c.credential_id), 1_700_200_000),
    ).resolves.toBe(false);
  }, 30000);

  it("denies a replay of an already-used token", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);
    const c = credential(
      "40000000-0000-0000-0000-000000000003",
      1_700_000_000,
      1_700_100_000,
    );
    await store.issue(c);

    await expect(
      store.claim(c.credential_id, secretFor(c.credential_id), 1_700_000_010),
    ).resolves.toBe(true);
    // The same token presented again is a replay.
    await expect(
      store.claim(c.credential_id, secretFor(c.credential_id), 1_700_000_020),
    ).resolves.toBe(false);
  }, 30000);

  it("rejects an invalid integration ladder leap with Policy", async () => {
    const db = await freshDb(stack);
    const store = new IntegrationStateStore(db);
    const integrationId = "40000000-0000-0000-0000-000000000010";
    await store.create(integrationId, "home-assistant", 1_700_000_000);

    // CONFIGURED -> HEALTHY is an invalid leap (credential-exists never
    // implies HEALTHY). The store must refuse with Policy.
    const request = IntegrationCardRequest.parse({
      integration_id: integrationId,
      provider_name: "home-assistant",
      correlation_id: "40000000-0000-0000-0000-000000000011",
    });
    await expect(
      store.recordStatus(
        integrationId,
        request,
        "HEALTHY",
        1_700_000_010,
        "40000000-0000-0000-0000-000000000011",
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Policy });

    // The durable row is unchanged: still UNCONFIGURED.
    const row = await store.read(
      integrationId,
      "40000000-0000-0000-0000-000000000011",
    );
    expect(row?.status).toBe("UNCONFIGURED");
  }, 30000);

  it("refuses a retry-safe recovery for an unknown mutation (Policy)", async () => {
    const db = await freshDb(stack);
    // Direct durable proof: the SQL CHECK refuses UNKNOWN + retry_safe.
    await expect(
      db.query(
        `INSERT INTO onboarding_recovery_checkpoint
           (checkpoint_id, mutation_id, mutation_kind, mutation_state,
            failure_class, outcome, retry_safe, created_at_unix_s, detail,
            correlation_id)
         VALUES ($1, 'unknown-mutation', 'owner_bootstrap', 'UNKNOWN',
                 'AMBIGUOUS', 'RETRYABLE', TRUE, 100, 'blind retry attempt', $2)`,
        [
          "40000000-0000-0000-0000-000000000020",
          "40000000-0000-0000-0000-000000000021",
        ],
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Validation });
  }, 30000);
});
