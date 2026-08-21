/**
 * EP-035 M3 onboarding event publisher (NATS JetStream transport).
 *
 * Real event emission for the onboarding lifecycle: owner bootstrap,
 * deployment intent, enrollment, integration state. Payloads are
 * redacted before publish: secrets never cross the event bus.
 */

import {
  connect,
  type NatsConnection,
  type JetStreamClient,
  type JetStreamPublishOptions,
} from "nats";
import { ErrorCode, Spec006Error } from "@nexus/setup";
import { redactSecrets } from "./redact.js";

export const ONBOARDING_EVENT_SUBJECTS = {
  owner_initialized: "nexus.onboarding.owner.initialized",
  owner_conflict: "nexus.onboarding.owner.conflict",
  deployment_selected: "nexus.onboarding.deployment.selected",
  deployment_verified: "nexus.onboarding.deployment.verified",
  enrollment_issued: "nexus.onboarding.enrollment.issued",
  enrollment_claimed: "nexus.onboarding.enrollment.claimed",
  integration_status: "nexus.onboarding.integration.status",
  recovery_checkpoint: "nexus.onboarding.recovery.checkpoint",
} as const;

export type OnboardingEventKind = keyof typeof ONBOARDING_EVENT_SUBJECTS;

export interface OnboardingEventPayload {
  correlation_id: string;
  occurred_at_unix_s: number;
  [key: string]: unknown;
}

export class OnboardingEventPublisher {
  private nc: NatsConnection | undefined;
  private js: JetStreamClient | undefined;

  constructor(
    private readonly url: string,
    private readonly streamName = "NEXUS_ONBOARDING",
  ) {}

  /** Connect to the real NATS server and ensure the onboarding stream exists. */
  async connect(): Promise<void> {
    try {
      this.nc = await connect({ servers: this.url, timeout: 5000 });
      this.js = this.nc.jetstream();
      try {
        // JetStream publish requires a stream for the subject prefix.
        // Idempotent: already-exists is fine (nats.js 2.29.3: stream
        // management lives on jetstreamManager(), not the js client).
        const jsm = await this.nc.jetstreamManager();
        await jsm.streams.add({
          name: this.streamName,
          subjects: ["nexus.onboarding.>"],
        });
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        if (!msg.includes("already exists") && !msg.includes("10058")) {
          throw err;
        }
      }
    } catch (err) {
      if (err instanceof Spec006Error) {
        throw err;
      }
      throw new Spec006Error(ErrorCode.Unavailable, "NATS connection refused");
    }
  }

  get connected(): boolean {
    return this.nc !== undefined && !this.nc.isClosed();
  }

  /**
   * Publish a redacted onboarding event. The payload is serialized with
   * secrets removed; only correlation, state, and safe summaries cross
   * the boundary.
   */
  async publish(
    kind: OnboardingEventKind,
    payload: OnboardingEventPayload,
    options?: JetStreamPublishOptions,
  ): Promise<string> {
    if (!this.connected || this.js === undefined) {
      throw new Spec006Error(ErrorCode.Unavailable, "NATS not connected");
    }
    const subject = ONBOARDING_EVENT_SUBJECTS[kind];
    const safe = redactSecrets(JSON.stringify(payload));
    try {
      const ack = await this.js.publish(
        subject,
        new TextEncoder().encode(safe),
        options,
      );
      return String(ack.seq);
    } catch (err) {
      if (err instanceof Spec006Error) {
        throw err;
      }
      throw new Spec006Error(ErrorCode.External, "NATS publish failed");
    }
  }

  /** Close the connection. */
  async close(): Promise<void> {
    if (this.nc !== undefined) {
      await this.nc.close();
      this.nc = undefined;
      this.js = undefined;
    }
  }
}
