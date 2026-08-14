// Connector SDK surface (SPEC-022 behavior 4) - TypeScript binding.
//
// Mirrors the Rust `ConnectorSdk` trait: the shared contract corpus
// that every language SDK implements. Field names are canonical
// snake_case wire names (matching the generated bindings and the Rust
// contract), so a request serialized by this SDK is byte-compatible
// with the same request serialized by the Rust SDK.

import type {
  CapabilityDescriptor,
  InvocationContext,
} from "@nexus/contracts";

// Re-export the canonical generated types so SDK consumers do not
// need to depend on @nexus/contracts directly for the SDK surface.
export type { CapabilityDescriptor, InvocationContext } from "@nexus/contracts";

import type { SdkError } from "./error.js";
import { sdkError } from "./error.js";
import type { SdkLanguage } from "./vocabulary.js";

/** One typed query request (SPEC-003/SPEC-022). */
export interface QueryRequest {
  capability_id: string;
  input: Record<string, unknown>;
  context: InvocationContext;
}

/** One typed query result. */
export interface QueryResult {
  capability_id: string;
  output: Record<string, unknown>;
}

/** One idempotent command request (SPEC-006/SPEC-022 behavior 2). */
export interface CommandRequest {
  capability_id: string;
  input: Record<string, unknown>;
  idempotency_key?: string;
  context: InvocationContext;
}

/** One typed command result. */
export interface CommandResult {
  capability_id: string;
  output: Record<string, unknown>;
}

/** Health observation (never an authorization claim). */
export interface HealthReport {
  target_id: string;
  state: "HEALTHY" | "DEGRADED" | "UNAVAILABLE" | "UNKNOWN";
  detail?: string;
}

/** One change-feed event (SPEC-022 behavior 8). */
export interface ChangeEvent {
  event_id: string;
  event_type: string;
  version: string;
  correlation_id: string;
  payload: Record<string, unknown>;
}

/** One change-feed batch with a stable cursor. */
export interface ChangeBatch {
  capability_id: string;
  events: ChangeEvent[];
  next_cursor: string;
}

/**
 * Port for a capability provider. TypeScript SDK implementations wrap
 * a concrete provider behind this port; the SDK never talks to a
 * provider directly.
 */
export interface QueryCapabilityPort {
  query(request: QueryRequest): Promise<QueryResult>;
}

export interface CommandCapabilityPort {
  command(request: CommandRequest): Promise<CommandResult>;
}

export interface HealthCapabilityPort {
  health(context: InvocationContext): Promise<HealthReport>;
}

export interface ChangeFeedCapabilityPort {
  changes_since(
    capability_id: string,
    cursor: string | undefined,
    context: InvocationContext,
  ): Promise<ChangeBatch>;
}

/** Idempotency record (SPEC-006). */
export interface IdempotencyRecord {
  key: string;
  capability_id: string;
  result: Record<string, unknown>;
}

/**
 * Deterministic idempotency tracker mirroring the Rust
 * `IdempotencyTracker`: a key is bound to the capability it was first
 * used with; reusing a key for a different capability is a conflict.
 */
export class IdempotencyTracker {
  private readonly records = new Map<string, IdempotencyRecord>();

  record(record: IdempotencyRecord): SdkError | undefined {
    const existing = this.records.get(record.key);
    if (existing !== undefined && existing.capability_id !== record.capability_id) {
      return sdkError(
        "CONFLICT",
        "idempotency key reused for a different capability",
        { resource: record.capability_id },
      );
    }
    this.records.set(record.key, record);
    return undefined;
  }

  get(key: string): IdempotencyRecord | undefined {
    return this.records.get(key);
  }

  get size(): number {
    return this.records.size;
  }
}

/**
 * The shared connector SDK contract (SPEC-022 behavior 4). Every
 * language binding implements this surface; the same conformance
 * corpus must pass against each implementation.
 */
export class ConnectorSdk {
  /** Language of this binding. */
  readonly language: SdkLanguage = "TYPESCRIPT";

  /** Shared contract corpus version. */
  readonly contractVersion: string = "1.0.0";

  private readonly queryPorts = new Map<string, QueryCapabilityPort>();
  private readonly commandPorts = new Map<string, CommandCapabilityPort>();
  private readonly healthPorts = new Map<string, HealthCapabilityPort>();
  private readonly feedPorts = new Map<string, ChangeFeedCapabilityPort>();
  private readonly tracker: IdempotencyTracker;
  private readonly descriptors = new Map<string, CapabilityDescriptor>();

  constructor(tracker?: IdempotencyTracker) {
    this.tracker = tracker ?? new IdempotencyTracker();
  }

  /** Register a capability descriptor (tenant-scoped advertisement). */
  registerDescriptor(descriptor: CapabilityDescriptor): void {
    this.descriptors.set(descriptor.id, descriptor);
  }

  /** Register a typed capability port. */
  registerQuery(capability_id: string, port: QueryCapabilityPort): void {
    this.queryPorts.set(capability_id, port);
  }

  registerCommand(capability_id: string, port: CommandCapabilityPort): void {
    this.commandPorts.set(capability_id, port);
  }

  registerHealth(capability_id: string, port: HealthCapabilityPort): void {
    this.healthPorts.set(capability_id, port);
  }

  registerChangeFeed(capability_id: string, port: ChangeFeedCapabilityPort): void {
    this.feedPorts.set(capability_id, port);
  }

  /** Discover advertised capabilities (metadata only). */
  discover(_context: InvocationContext): CapabilityDescriptor[] {
    return [...this.descriptors.values()].filter(
      (d) => d.availability === "AVAILABLE",
    );
  }

  /** Execute a typed query. */
  async query(request: QueryRequest): Promise<QueryResult> {
    const descriptor = this.descriptors.get(request.capability_id);
    if (descriptor === undefined) {
      throw sdkError("NOT_FOUND", "capability not found", {
        correlationId: request.context.correlation_id,
        resource: request.capability_id,
      });
    }
    if (descriptor.class !== "QUERY") {
      throw sdkError("VALIDATION", "capability is not a QUERY class", {
        correlationId: request.context.correlation_id,
        resource: request.capability_id,
      });
    }
    const port = this.queryPorts.get(request.capability_id);
    if (port === undefined) {
      throw sdkError("UNAVAILABLE", "no query provider registered", {
        correlationId: request.context.correlation_id,
        resource: request.capability_id,
      });
    }
    return port.query(request);
  }

  /** Execute an idempotent command (SPEC-006/SPEC-022 behavior 2). */
  async command(request: CommandRequest): Promise<CommandResult> {
    const descriptor = this.descriptors.get(request.capability_id);
    if (descriptor === undefined) {
      throw sdkError("NOT_FOUND", "capability not found", {
        correlationId: request.context.correlation_id,
        resource: request.capability_id,
      });
    }
    if (descriptor.class !== "COMMAND") {
      throw sdkError("VALIDATION", "capability is not a COMMAND class", {
        correlationId: request.context.correlation_id,
        resource: request.capability_id,
      });
    }
    const port = this.commandPorts.get(request.capability_id);
    if (port === undefined) {
      throw sdkError("UNAVAILABLE", "no command provider registered", {
        correlationId: request.context.correlation_id,
        resource: request.capability_id,
      });
    }
    if (request.idempotency_key !== undefined) {
      const existing = this.tracker.get(request.idempotency_key);
      if (existing !== undefined && existing.capability_id === request.capability_id) {
        // Replay the recorded result; the provider is not invoked again.
        return {
          capability_id: request.capability_id,
          output: existing.result,
        };
      }
    }
    const result = await port.command(request);
    if (request.idempotency_key !== undefined) {
      const conflict = this.tracker.record({
        key: request.idempotency_key,
        capability_id: result.capability_id,
        result: result.output,
      });
      if (conflict !== undefined) {
        throw conflict;
      }
    }
    return result;
  }

  /** Read capability health (observation only). */
  async health(capability_id: string, context: InvocationContext): Promise<HealthReport> {
    const descriptor = this.descriptors.get(capability_id);
    if (descriptor === undefined) {
      throw sdkError("NOT_FOUND", "capability not found", {
        correlationId: context.correlation_id,
        resource: capability_id,
      });
    }
    if (descriptor.availability !== "AVAILABLE") {
      throw sdkError("UNAVAILABLE", "capability is not available", {
        correlationId: context.correlation_id,
        resource: capability_id,
      });
    }
    const port = this.healthPorts.get(capability_id);
    if (port === undefined) {
      throw sdkError("UNAVAILABLE", "no health provider registered", {
        correlationId: context.correlation_id,
        resource: capability_id,
      });
    }
    return port.health(context);
  }

  /** Read change-feed events (SPEC-022 behavior 8). */
  async changefeed(
    capability_id: string,
    cursor: string | undefined,
    context: InvocationContext,
  ): Promise<ChangeBatch> {
    const descriptor = this.descriptors.get(capability_id);
    if (descriptor === undefined) {
      throw sdkError("NOT_FOUND", "capability not found", {
        correlationId: context.correlation_id,
        resource: capability_id,
      });
    }
    const port = this.feedPorts.get(capability_id);
    if (port === undefined) {
      throw sdkError("UNAVAILABLE", "no change-feed provider registered", {
        correlationId: context.correlation_id,
        resource: capability_id,
      });
    }
    return port.changes_since(capability_id, cursor, context);
  }
}
