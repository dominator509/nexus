/**
 * EP-035 M4 forced failure: observability and incident correlation.
 *
 * The REAL observability surface: structured SPEC-006 errors carry the
 * correlation id, redaction strips secret-shaped material from error
 * detail, and the NATS event bus carries redacted payloads (ZERO
 * LEAKAGE canary) so an incident record never becomes a secret leak.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { ErrorCode, Spec006Error } from "@nexus/setup";
import {
  OnboardingEventPublisher,
  OnboardingDb,
  redactErrorDetail,
  redactSecrets,
} from "@nexus/onboarding";
import { freshDb, startStack, stopStack, type TestStack } from "./harness.js";
import { connect } from "nats";
import { spawnSync } from "node:child_process";

function hostPortForDead(): number {
  // Reserve an ephemeral port then release it: nothing listens there.
  const res = spawnSync("python3", [
    "-c",
    "import socket; s=socket.socket(); s.bind(('127.0.0.1',0)); print(s.getsockname()[1]); s.close()",
  ]);
  return Number(res.stdout.toString().trim());
}

describe("ep035_failure_observability", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  it("carries the correlation id in structured errors", async () => {
    const db = await freshDb(stack);
    // Violate a constraint with a correlation id; the mapped error must
    // preserve the incident correlation.
    const corr = "60000000-0000-0000-0000-000000000001";
    try {
      await db.query(
        `INSERT INTO onboarding_owner
           (owner_id, idempotency_key, owner_email, state, correlation_id,
            created_at_unix_s, updated_at_unix_s)
         VALUES ($1, 'obs-dup-key', 'obs@nexus.test', 'OWNER_PRINCIPAL_CREATED', $2, 1, 1)`,
        ["60000000-0000-0000-0000-000000000002", corr],
        corr,
      );
      await db.query(
        `INSERT INTO onboarding_owner
           (owner_id, idempotency_key, owner_email, state, correlation_id,
            created_at_unix_s, updated_at_unix_s)
         VALUES ($1, 'obs-dup-key', 'obs@nexus.test', 'OWNER_PRINCIPAL_CREATED', $2, 1, 1)`,
        ["60000000-0000-0000-0000-000000000003", corr],
        corr,
      );
      throw new Error("expected duplicate to fail");
    } catch (err) {
      const spec = err as Spec006Error;
      expect(spec.code).toBe(ErrorCode.Conflict);
      expect(spec.correlationId).toBe(corr);
    }
  }, 30000);

  it("redacts secret-shaped material from error detail", () => {
    const canary = "nexus_secret_canary_abcdefghijklmnopqrstuvwxyz012345";
    const detail = `connect failed with credential ${canary} at 10.0.0.1`;
    const safe = redactErrorDetail(detail);
    expect(safe).not.toContain(canary);
    expect(safe).toContain("[REDACTED]");
    // A correlation id is a safe field and stays readable.
    const withCorr = redactSecrets(
      `correlation=60000000-0000-0000-0000-000000000010`,
    );
    expect(withCorr).toContain("60000000-0000-0000-0000-000000000010");
  });

  it("publishes redacted events with zero leakage on the real bus", async () => {
    const db = await freshDb(stack);
    const url = `nats://127.0.0.1:${stack.natsPort}`;
    const publisher = new OnboardingEventPublisher(url, "NEXUS_ONBOARDING_M4");
    await publisher.connect();

    const sub = await connect({ servers: url });
    const inbox = sub.subscribe("nexus.onboarding.owner.initialized");
    // Attach the iterator BEFORE publishing; flush so the SUB protocol
    // message reaches the server before the publish.
    const iter = inbox[Symbol.asyncIterator]();
    await sub.flush();

    const canary = "nexus_token_canary_abcdefghijklmnopqrstuvwxyz012345";
    await publisher.publish("owner_initialized", {
      correlation_id: "60000000-0000-0000-0000-000000000020",
      occurred_at_unix_s: 1_700_000_000,
      owner_id: "60000000-0000-0000-0000-000000000021",
      bootstrap_secret: canary,
    });

    const first = await iter.next();
    expect(first.done).toBe(false);
    const body = new TextDecoder().decode(first.value.data);
    expect(body).not.toContain(canary);
    expect(body).toContain("[REDACTED]");
    expect(body).toContain("60000000-0000-0000-0000-000000000020");

    await sub.close();
    await publisher.close();
  }, 30000);

  it("maps a refused NATS connection to Unavailable (SPEC-006)", async () => {
    // A port with nothing listening: real connection refused.
    const dead = new OnboardingEventPublisher(
      `nats://127.0.0.1:${hostPortForDead()}`,
    );
    await expect(dead.connect()).rejects.toMatchObject({
      code: ErrorCode.Unavailable,
    });
  }, 30000);
});
