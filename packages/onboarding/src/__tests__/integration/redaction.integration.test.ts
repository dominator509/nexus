/**
 * EP-035 M3 integration: redaction canaries across the real boundary.
 *
 * Inject canary secrets into every onboarding secret slot and assert
 * ZERO_LEAKAGE: errors, event payloads, evidence records, and log-shaped
 * strings never contain the raw canary material.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { randomUUID } from "node:crypto";
import { EnrollmentCredential } from "@nexus/setup";
import { EnrollmentTokenStore } from "../../stores/enrollment-token.store.js";
import { redactErrorDetail, redactSecrets, safeSummary } from "../../redact.js";
import {
  OnboardingEventPublisher,
  ONBOARDING_EVENT_SUBJECTS,
} from "../../events.js";
import {
  freshDb,
  hostPort,
  run,
  startStack,
  stopStack,
  type TestStack,
} from "./harness.js";
import { connect, type NatsConnection } from "nats";

const CANARY = `bootstrap_secret_${"C".repeat(32)}`;
const CANARY_NONCE = `nonce_${"N".repeat(32)}`;

describe("ep035_integration_redaction", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  it("redacts secret-shaped content from summaries and error detail", () => {
    const summary = safeSummary({
      credential_id: randomUUID(),
      secret: CANARY,
      nonce: CANARY_NONCE,
      state: "ISSUED",
    });
    expect(summary).not.toContain(CANARY);
    expect(summary).not.toContain(CANARY_NONCE);

    const detail = redactErrorDetail(
      `claim failed for secret ${CANARY} nonce ${CANARY_NONCE}`,
    );
    expect(detail).not.toContain(CANARY);
    expect(detail).not.toContain(CANARY_NONCE);

    expect(redactSecrets(`token=${CANARY} rest`)).not.toContain(CANARY);
  });

  it("never stores or emits the raw enrollment secret (canary in DB + redacted shape)", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);
    const now = 1_700_000_000;

    const credential = EnrollmentCredential.parse({
      credential_id: randomUUID(),
      kind: "BOOTSTRAP_TOKEN",
      issued_at_unix_s: now,
      expires_at_unix_s: now + 3600,
      state: "ISSUED",
      nonce: CANARY_NONCE,
      secret: CANARY,
    });

    const issued = await store.issue(credential);
    const json = JSON.stringify(issued);
    expect(json).not.toContain(CANARY);
    expect(json).not.toContain(CANARY_NONCE);

    // The DB row stores only hashes.
    const row = await store.read(credential.credential_id);
    const rowJson = JSON.stringify(row);
    expect(rowJson).not.toContain(CANARY);
    expect(rowJson).not.toContain(CANARY_NONCE);

    await db.close();
  });

  it("redacts canaries in published NATS event payloads (ZERO_LEAKAGE on the bus)", async () => {
    const publisher = new OnboardingEventPublisher(
      `nats://127.0.0.1:${stack.natsPort}`,
    );
    await publisher.connect();
    try {
      const nc: NatsConnection = await connect({
        servers: `nats://127.0.0.1:${stack.natsPort}`,
        timeout: 5000,
      });
      const sub = nc.subscribe(ONBOARDING_EVENT_SUBJECTS.owner_initialized);
      // Attach the iterator BEFORE publishing (deterministic delivery).
      const iter = sub[Symbol.asyncIterator]();
      // Flush so the SUB protocol message reaches the server before the
      // publish (canonical NATS guarantee under load).
      await nc.flush();

      await publisher.publish("owner_initialized", {
        correlation_id: randomUUID(),
        occurred_at_unix_s: 1_700_000_000,
        principal_id: randomUUID(),
        // A buggy caller passes a secret; the publisher must redact it.
        bootstrap_secret: CANARY,
        nonce: CANARY_NONCE,
      });

      const first = await iter.next();
      expect(first.done).toBe(false);
      const payload = new TextDecoder().decode(first.value.data);
      expect(payload).not.toContain(CANARY);
      expect(payload).not.toContain(CANARY_NONCE);
      expect(payload).toContain("principal_id");

      await nc.close();
    } finally {
      await publisher.close();
    }
  }, 30000);
});
