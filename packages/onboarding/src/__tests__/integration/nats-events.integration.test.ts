/**
 * EP-035 M3 integration: real NATS JetStream event emission.
 *
 * Onboarding lifecycle events cross the real event bus with redacted
 * payloads. Proves connect, publish (JetStream ack), and subscribe
 * round-trip against the real nats:2.14.3 container.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { randomUUID } from "node:crypto";
import { connect, type NatsConnection } from "nats";
import {
  OnboardingEventPublisher,
  ONBOARDING_EVENT_SUBJECTS,
} from "../../events.js";
import { startStack, stopStack, type TestStack } from "./harness.js";

describe("ep035_integration_nats_events", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  it("publishes owner_initialized and a subscriber receives the redacted event", async () => {
    const publisher = new OnboardingEventPublisher(
      `nats://127.0.0.1:${stack.natsPort}`,
    );
    await publisher.connect();
    let nc: NatsConnection | undefined;
    try {
      nc = await connect({
        servers: `nats://127.0.0.1:${stack.natsPort}`,
        timeout: 5000,
      });
      const sub = nc.subscribe(ONBOARDING_EVENT_SUBJECTS.owner_initialized);
      // Attach the iterator BEFORE publishing: a message that arrives
      // before the iterator attaches can be missed under load.
      const iter = sub[Symbol.asyncIterator]();
      // Flush so the SUB protocol message reaches the server before the
      // publish (canonical NATS guarantee under load).
      await nc.flush();

      const seq = await publisher.publish("owner_initialized", {
        correlation_id: randomUUID(),
        occurred_at_unix_s: 1_700_000_000,
        principal_id: randomUUID(),
      });
      expect(seq).toBeTruthy();

      const first = await iter.next();
      expect(first.done).toBe(false);
      const payload = JSON.parse(new TextDecoder().decode(first.value.data));
      expect(payload.correlation_id).toBeDefined();
      expect(payload.principal_id).toBeDefined();
      expect(payload.secret).toBeUndefined();
    } finally {
      await nc?.close();
      await publisher.close();
    }
  }, 30000);

  it("emits enrollment, deployment, integration, and recovery subjects", async () => {
    const publisher = new OnboardingEventPublisher(
      `nats://127.0.0.1:${stack.natsPort}`,
    );
    await publisher.connect();
    let nc: NatsConnection | undefined;
    try {
      nc = await connect({
        servers: `nats://127.0.0.1:${stack.natsPort}`,
        timeout: 5000,
      });
      const subs = [
        ONBOARDING_EVENT_SUBJECTS.enrollment_claimed,
        ONBOARDING_EVENT_SUBJECTS.deployment_selected,
        ONBOARDING_EVENT_SUBJECTS.integration_status,
        ONBOARDING_EVENT_SUBJECTS.recovery_checkpoint,
      ].map((subject) => nc!.subscribe(subject));
      // Attach iterators BEFORE publishing (deterministic delivery).
      const iters = subs.map((sub) => sub[Symbol.asyncIterator]());
      // Flush so all SUB protocol messages reach the server before any
      // publish (canonical NATS guarantee under load).
      await nc!.flush();

      const correlation = randomUUID();
      const seqs = await Promise.all([
        publisher.publish("enrollment_claimed", {
          correlation_id: correlation,
          occurred_at_unix_s: 1_700_000_000,
          credential_id: randomUUID(),
        }),
        publisher.publish("deployment_selected", {
          correlation_id: correlation,
          occurred_at_unix_s: 1_700_000_000,
          mode: "FULLY_LOCAL",
        }),
        publisher.publish("integration_status", {
          correlation_id: correlation,
          occurred_at_unix_s: 1_700_000_000,
          status: "CONFIGURED",
        }),
        publisher.publish("recovery_checkpoint", {
          correlation_id: correlation,
          occurred_at_unix_s: 1_700_000_000,
          outcome: "RECONCILE",
        }),
      ]);
      seqs.forEach((seq) => expect(seq).toBeTruthy());

      for (const iter of iters) {
        const first = await iter.next();
        expect(first.done).toBe(false);
        const payload = JSON.parse(new TextDecoder().decode(first.value.data));
        expect(payload.correlation_id).toBe(correlation);
      }
    } finally {
      await nc?.close();
      await publisher.close();
    }
  }, 30000);

  it("fails closed with Unavailable when NATS is not connected", async () => {
    const publisher = new OnboardingEventPublisher(
      "nats://127.0.0.1:1", // nothing listens here
    );
    await expect(
      publisher.publish("owner_initialized", {
        correlation_id: randomUUID(),
        occurred_at_unix_s: 1_700_000_000,
      }),
    ).rejects.toMatchObject({ code: "UNAVAILABLE" });
    await publisher.close();
  });
});
