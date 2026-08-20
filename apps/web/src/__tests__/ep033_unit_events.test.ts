import { describe, expect, it } from "vitest";
import { EventFilter, EventSubscription } from "../contracts/events";
import { ErrorCode, Spec006Error } from "../contracts/errors";
import type { EventEnvelope } from "@nexus/contracts";

function envelope(overrides: Partial<EventEnvelope> = {}): EventEnvelope {
  return {
    event_id: "evt-0001",
    event_type: "objective.stage_changed",
    schema_version: "1.0.0",
    source: "objectives",
    subject: "obj-0001",
    time: "2026-08-20T00:00:00Z",
    tenant_id: "00000000-0000-4000-8000-000000000003",
    actor: "00000000-0000-4000-8000-000000000002",
    correlation_id: "corr-0001",
    data_class: "PERSONAL",
    payload: { stage: "ACTIVE" },
    ...overrides,
  };
}

describe("ep033_unit_events_subscription", () => {
  it("constructs a typed event filter", () => {
    const filter = EventFilter.fromWire({
      event_type: "objective.stage_changed",
      source: "objectives",
      schema_version: "1.0.0",
      correlation: "corr-0001",
    });
    expect(filter.describe()).toBe("objectives:objective.stage_changed@v1.0.0");
  });

  it("matches canonical event envelopes by type, source, and version", () => {
    const filter = EventFilter.fromWire({
      event_type: "objective.stage_changed",
      source: "objectives",
      schema_version: "1.0.0",
      correlation: "corr-0001",
    });
    expect(filter.matches(envelope())).toBe(true);
    expect(filter.matches(envelope({ event_type: "objective.created" }))).toBe(false);
    expect(filter.matches(envelope({ source: "memory" }))).toBe(false);
    expect(
      filter.matches(envelope({ schema_version: "2.0.0" } as unknown as Partial<EventEnvelope>)),
    ).toBe(false);
  });

  it("rejects empty event types and sources", () => {
    expect(() =>
      EventFilter.fromWire({
        event_type: "",
        source: "objectives",
        schema_version: "1.0.0",
        correlation: "corr-0001",
      }),
    ).toThrowError(Spec006Error);
  });

  it("rejects unknown fields", () => {
    expect(() =>
      EventFilter.fromWire({
        event_type: "objective.stage_changed",
        source: "objectives",
        schema_version: "1.0.0",
        correlation: "corr-0001",
        payload_filter: {},
      }),
    ).toThrowError(Spec006Error);
  });

  it("accepts a broadcast event when its filter matches", () => {
    const subscription = new EventSubscription(
      EventFilter.fromWire({
        event_type: "objective.stage_changed",
        source: "objectives",
        schema_version: "1.0.0",
        correlation: "corr-0001",
      }),
      "corr-0001",
    );
    expect(subscription.accept(envelope())).toBe(true);
    expect(subscription.accept(envelope({ event_type: "unrelated.event" }))).toBe(false);
  });
});
