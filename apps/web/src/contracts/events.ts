/**
 * EP-033 M1 event subscription contract (directive H).
 *
 * Dashboard realtime surfaces subscribe to canonical EventEnvelope
 * events. Subscriptions are typed: event_type and source come from
 * known vocabulary, schema_version is validated, and a subscription
 * never invents event types.
 */

import type { EventEnvelope } from "@nexus/contracts";
import {
  assertObject,
  assertString,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const EVENT_FILTER_FIELDS = new Set<string>([
  "event_type",
  "source",
  "schema_version",
  "correlation",
]);

export interface EventFilterShape {
  event_type: string;
  source: string;
  schema_version: string;
  correlation: string;
}

/** A typed subscription filter over canonical event vocabulary. */
export class EventFilter {
  readonly event_type: string;
  readonly source: string;
  readonly schema_version: string;
  readonly correlation: string;

  private constructor(shape: EventFilterShape) {
    this.event_type = shape.event_type;
    this.source = shape.source;
    this.schema_version = shape.schema_version;
    this.correlation = shape.correlation;
  }

  static fromWire(value: unknown): EventFilter {
    const obj = assertObject(value, "EventFilter");
    rejectUnknownFields(obj, EVENT_FILTER_FIELDS, "EventFilter");
    const eventType = assertString(obj.event_type, "event_type");
    const source = assertString(obj.source, "source");
    if (eventType.length === 0 || source.length === 0) {
      throw new Spec006Error(ErrorCode.Validation, "event_type and source must not be empty");
    }
    return new EventFilter({
      event_type: eventType,
      source,
      schema_version: assertString(obj.schema_version, "schema_version"),
      correlation: assertString(obj.correlation, "correlation"),
    });
  }

  /**
   * Match a canonical event envelope. Unknown event types on the wire
   * do not match and are surfaced as unsupported, never fabricated
   * into the subscription stream.
   */
  matches(envelope: EventEnvelope): boolean {
    if (envelope.event_type !== this.event_type) {
      return false;
    }
    if (envelope.source !== this.source) {
      return false;
    }
    return envelope.schema_version === this.schema_version;
  }

  /** A subscription must be able to name what it will receive. */
  describe(): string {
    return `${this.source}:${this.event_type}@v${this.schema_version}`;
  }
}

/** Canonical event subscription record (typed, versioned). */
export class EventSubscription {
  readonly filter: EventFilter;
  readonly correlation: string;

  constructor(filter: EventFilter, correlation: string) {
    this.filter = filter;
    this.correlation = correlation;
  }

  /** A broadcast event is accepted when its filter matches. */
  accept(envelope: EventEnvelope): boolean {
    return this.filter.matches(envelope);
  }
}
