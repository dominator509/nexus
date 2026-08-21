/**
 * EP-035 M3 integration: real failure classification across the boundary.
 *
 * Transport failures map to the canonical SPEC-006 vocabulary, never
 * collapsed to a generic External: connection refused -> UNAVAILABLE,
 * connect timeout -> TIMEOUT, malformed input -> VALIDATION, duplicate
 * value -> CONFLICT, auth denied -> AUTHENTICATION.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { ErrorCode } from "@nexus/setup";
import { OnboardingDb } from "../../db.js";
import { startStack, stopStack, type TestStack } from "./harness.js";
import { spawnSync } from "node:child_process";

function unusedPort(): number {
  // Reserve an ephemeral port then release it: nothing listens there.
  const res = spawnSync("python3", [
    "-c",
    "import socket; s=socket.socket(); s.bind(('127.0.0.1',0)); print(s.getsockname()[1]); s.close()",
  ]);
  return Number(res.stdout.toString().trim());
}

describe("ep035_integration_failures", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  it("maps connection refused to Unavailable (SPEC-006)", async () => {
    const db = new OnboardingDb({
      host: "127.0.0.1",
      port: unusedPort(),
      user: "nexus",
      password: "nexus-test",
      database: "nexus",
      connectionTimeoutMillis: 2000,
    });
    try {
      await expect(db.query("SELECT 1")).rejects.toMatchObject({
        code: ErrorCode.Unavailable,
      });
    } finally {
      await db.close();
    }
  });

  it("maps connect timeout to Timeout (SPEC-006)", async () => {
    // 10.255.255.1 is a non-routable test address: connections hang
    // until the client's own connect timeout fires.
    const db = new OnboardingDb({
      host: "10.255.255.1",
      port: 5432,
      user: "nexus",
      password: "nexus-test",
      database: "nexus",
      connectionTimeoutMillis: 1500,
    });
    try {
      await expect(db.query("SELECT 1")).rejects.toMatchObject({
        code: ErrorCode.Timeout,
      });
    } finally {
      await db.close();
    }
  });

  it("maps database authentication denial to Authentication (SPEC-006)", async () => {
    const db = new OnboardingDb({
      host: "127.0.0.1",
      port: stack.pgPort,
      user: "nexus",
      password: "definitely-wrong-password",
      database: "nexus",
      connectionTimeoutMillis: 3000,
    });
    try {
      await expect(db.query("SELECT 1")).rejects.toMatchObject({
        code: ErrorCode.Authentication,
      });
    } finally {
      await db.close();
    }
  });

  it("maps malformed stored input to Validation (SPEC-006)", async () => {
    // Insert a row violating the enrollment CHECK constraint directly.
    await expect(
      stack.db.query(
        `INSERT INTO onboarding_enrollment_credential
           (credential_id, kind, state, issued_at_unix_s, expires_at_unix_s,
            secret_hash, nonce_hash, correlation_id)
         VALUES ($1, 'BOOTSTRAP_TOKEN', 'USED', 100, 200, 'h', 'h', $2)`,
        [
          "00000000-0000-0000-0000-000000000001",
          "00000000-0000-0000-0000-000000000002",
        ],
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Validation });
  });

  it("maps duplicate value to Conflict (SPEC-006)", async () => {
    await expect(
      stack.db.query(
        `INSERT INTO onboarding_owner
           (owner_id, idempotency_key, owner_email, state, correlation_id,
            created_at_unix_s, updated_at_unix_s)
         VALUES ($1, 'same-dup-key', 'dup@nexus.test', 'OWNER_PRINCIPAL_CREATED', $2, 1, 1)`,
        [
          "00000000-0000-0000-0000-000000000010",
          "00000000-0000-0000-0000-000000000011",
        ],
      ),
    ).resolves.toBeDefined();
    await expect(
      stack.db.query(
        `INSERT INTO onboarding_owner
           (owner_id, idempotency_key, owner_email, state, correlation_id,
            created_at_unix_s, updated_at_unix_s)
         VALUES ($1, 'same-dup-key', 'dup@nexus.test', 'OWNER_PRINCIPAL_CREATED', $2, 1, 1)`,
        [
          "00000000-0000-0000-0000-000000000012",
          "00000000-0000-0000-0000-000000000013",
        ],
      ),
    ).rejects.toMatchObject({ code: ErrorCode.Conflict });
  });
});
