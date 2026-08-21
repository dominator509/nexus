/**
 * EP-035 M4 forced failure: malformed input and corrupted messages.
 *
 * The REAL failure mechanism is a corrupted controlled message: a
 * malformed row injected at the durable boundary, an invalid status
 * enum, and an invalid deployment verification request. The production
 * stores must reject with Validation (SPEC-006) - never silently
 * coerce and never fabricate a success.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { ErrorCode } from "@nexus/setup";
import {
  DeploymentIntentStore,
  IntegrationStateStore,
  OnboardingDb,
} from "@nexus/onboarding";
import { freshDb, startStack, stopStack, type TestStack } from "./harness.js";

describe("ep035_failure_malformed_input", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  it("rejects a corrupted enrollment row violating the CHECK (Validation)", async () => {
    const db = await freshDb(stack);
    // A USED credential must carry used_at; corrupt it by direct write.
    await expect(
      db.query(
        `INSERT INTO onboarding_enrollment_credential
           (credential_id, kind, state, issued_at_unix_s, expires_at_unix_s,
            secret_hash, nonce_hash, correlation_id)
         VALUES ($1, 'BOOTSTRAP_TOKEN', 'USED', 100, 200, 'h', 'h', $2)`,
        [
          "20000000-0000-0000-0000-000000000001",
          "20000000-0000-0000-0000-000000000002",
        ],
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Validation });
  });

  it("rejects an unknown integration status enum (Validation)", async () => {
    const db = await freshDb(stack);
    await expect(
      db.query(
        `INSERT INTO onboarding_integration_state
           (integration_id, provider_name, status, updated_at_unix_s,
            correlation_id)
         VALUES ($1, 'home-assistant', 'BOGUS', 100, $2)`,
        [
          "20000000-0000-0000-0000-000000000003",
          "20000000-0000-0000-0000-000000000004",
        ],
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Validation });
  });

  it("rejects a corrupt non-JSON capability payload at the durable boundary", async () => {
    const db = await freshDb(stack);
    const store = new IntegrationStateStore(db);
    await store.create(
      "20000000-0000-0000-0000-000000000005",
      "home-assistant",
      1_700_000_000,
    );
    // setCapabilities stores supplied data as supplied (never
    // name-derived), so a scalar is legal. The durable boundary still
    // rejects a payload that is not JSON at all.
    await expect(
      db.query(
        `UPDATE onboarding_integration_state
            SET capability_json = $2::jsonb
          WHERE integration_id = $1`,
        ["20000000-0000-0000-0000-000000000005", "{not-json"],
        "20000000-0000-0000-0000-000000000006",
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Validation });
    // Exact-target readback: stored capability_json stays the default.
    const row = await store.read(
      "20000000-0000-0000-0000-000000000005",
      "20000000-0000-0000-0000-000000000006",
    );
    expect(JSON.stringify(row?.capability_json)).toBe("[]");
  });

  it("rejects a deployment verification without evidence (SELECTED != VERIFIED)", async () => {
    const db = await freshDb(stack);
    const store = new DeploymentIntentStore(db);
    // A verification with no evidence record must fail closed; the
    // store persists SELECTED and the DDL blocks evidence-less VERIFIED.
    await expect(
      store.recordVerification(
        "20000000-0000-0000-0000-000000000007",
        {
          intent_id: "20000000-0000-0000-0000-000000000007",
          evidence: undefined,
        } as never,
        1_700_000_000,
        "20000000-0000-0000-0000-000000000008",
      ),
    ).rejects.toThrow();
    // Direct DDL proof: VERIFIED without evidence violates the CHECK.
    await expect(
      db.query(
        `INSERT INTO onboarding_deployment_intent
           (intent_id, mode, release_channel, profile_json, verification_state,
            selected_at_unix_s, correlation_id)
         VALUES ($1, 'MANAGED', 'STABLE', '{}'::jsonb, 'VERIFIED', 100, $2)`,
        [
          "20000000-0000-0000-0000-000000000009",
          "20000000-0000-0000-0000-000000000010",
        ],
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Validation });
  });
});
