import { describe, expect, it } from "vitest";
import type {
  CapabilityDescriptor,
  InvocationContext,
} from "@nexus/contracts";

import { sdkError, isTransient, type SdkError } from "../error.js";
import { parseSdkLanguage, parseSidecarTransport } from "../vocabulary.js";
import {
  ConnectorSdk,
  IdempotencyTracker,
  type CommandCapabilityPort,
  type CommandRequest,
  type HealthCapabilityPort,
  type QueryCapabilityPort,
  type QueryRequest,
} from "../sdk.js";
import { CONTRACT_VERSION } from "../index.js";

const ctx = (): InvocationContext => ({
  request_id: "018f0f6f-9c1e-7b6e-8000-000000000001",
  correlation_id: "018f0f6f-9c1e-7b6e-8000-000000000002",
  origin_system: "test",
  external_actor_id: "user:alice",
  external_actor_type: "HUMAN",
});

const descriptor = (
  id: string,
  cls: "QUERY" | "COMMAND" | "WORKFLOW" | "STREAM" | "ADMINISTRATIVE",
  availability: "AVAILABLE" | "DEGRADED" | "UNAVAILABLE" | "UNCERTIFIED" = "AVAILABLE",
): CapabilityDescriptor => ({
  id,
  version: "1.0.0",
  class: cls,
  description: "A deterministic test capability",
  input_schema: "schemas/invocation-context.schema.json",
  output_schema: "schemas/capability-descriptor.schema.json",
  required_scopes: ["test.scope"],
  risk: "R1",
  approval: "NONE",
  reversal: "NONE",
  idempotency: "NOT_APPLICABLE",
  availability,
  locality: "ANY",
  data_classes: ["HOUSEHOLD"],
  event_types: ["test.changed"],
  provider_id: "test-provider",
});

function buildSdk() {
  const sdk = new ConnectorSdk();
  sdk.registerDescriptor(descriptor("test.query", "QUERY"));
  sdk.registerDescriptor(descriptor("test.command", "COMMAND"));
  sdk.registerDescriptor(descriptor("test.hidden", "QUERY", "UNAVAILABLE"));
  const queryPort: QueryCapabilityPort = {
    async query(request: QueryRequest) {
      return { capability_id: request.capability_id, output: { state: "on" } };
    },
  };
  let commandCalls = 0;
  const commandPort: CommandCapabilityPort = {
    async command(request: CommandRequest) {
      commandCalls += 1;
      return { capability_id: request.capability_id, output: { applied: true } };
    },
  };
  const healthPort: HealthCapabilityPort = {
    async health() {
      return { target_id: "test.query", state: "HEALTHY" };
    },
  };
  sdk.registerQuery("test.query", queryPort);
  sdk.registerCommand("test.command", commandPort);
  sdk.registerHealth("test.query", healthPort);
  return { sdk, commandCalls: () => commandCalls };
}

describe("ep011_unit_sdk_vocabulary", () => {
  it("parses canonical values and rejects unknown", () => {
    expect(parseSdkLanguage("RUST")).toBe("RUST");
    expect(parseSdkLanguage("PYTHON")).toBe("PYTHON");
    expect(() => parseSdkLanguage("COBOL")).toThrow();
    expect(parseSidecarTransport("BROWSER")).toBe("BROWSER");
    expect(() => parseSidecarTransport("SMOKE_SIGNALS")).toThrow();
  });
});

describe("ep011_unit_sdk_error", () => {
  it("classifies transient failures", () => {
    const down: SdkError = sdkError("UNAVAILABLE", "down");
    const denied: SdkError = sdkError("AUTHORIZATION", "denied");
    expect(isTransient(down)).toBe(true);
    expect(isTransient(denied)).toBe(false);
  });

  it("serializes without secrets", () => {
    const err = sdkError("EXTERNAL_PROVIDER", "provider refused", {
      correlationId: "corr-1",
      actor: "user:alice",
      resource: "cap:ledger",
    });
    expect(err.code).toBe("EXTERNAL_PROVIDER");
    expect(err.message).not.toContain("secret");
  });
});

describe("ep011_unit_sdk_surface", () => {
  it("exposes the shared contract version", () => {
    const { sdk } = buildSdk();
    expect(sdk.language).toBe("TYPESCRIPT");
    expect(sdk.contractVersion).toBe(CONTRACT_VERSION);
    expect(CONTRACT_VERSION).toBe("1.0.0");
  });

  it("discovers advertised capabilities (metadata only)", () => {
    const { sdk } = buildSdk();
    const discovered = sdk.discover(ctx()).map((d) => d.id);
    expect(discovered).toContain("test.query");
    expect(discovered).toContain("test.command");
    expect(discovered).not.toContain("test.hidden");
  });

  it("executes a typed query", async () => {
    const { sdk } = buildSdk();
    const result = await sdk.query({
      capability_id: "test.query",
      input: {},
      context: ctx(),
    });
    expect(result.output).toEqual({ state: "on" });
  });

  it("rejects a command sent to the query path (class mismatch)", async () => {
    const { sdk } = buildSdk();
    await expect(
      sdk.query({ capability_id: "test.command", input: {}, context: ctx() }),
    ).rejects.toMatchObject({ code: "VALIDATION" });
  });

  it("rejects unknown capabilities", async () => {
    const { sdk } = buildSdk();
    await expect(
      sdk.query({ capability_id: "test.missing", input: {}, context: ctx() }),
    ).rejects.toMatchObject({ code: "NOT_FOUND" });
  });

  it("rejects unavailable capability on health", async () => {
    const { sdk } = buildSdk();
    await expect(sdk.health("test.hidden", ctx())).rejects.toMatchObject({
      code: "UNAVAILABLE",
    });
  });
});

describe("ep011_unit_sdk_idempotency", () => {
  it("replays the recorded result without invoking the provider twice", async () => {
    const { sdk, commandCalls } = buildSdk();
    const request: CommandRequest = {
      capability_id: "test.command",
      input: { on: true },
      idempotency_key: "op-1",
      context: ctx(),
    };
    const first = await sdk.command(request);
    const second = await sdk.command(request);
    expect(first.output).toEqual({ applied: true });
    expect(second.output).toEqual({ applied: true });
    expect(commandCalls()).toBe(1);
  });

  it("conflicts when a key is reused for a different capability", async () => {
    const sdk = new ConnectorSdk();
    sdk.registerDescriptor(descriptor("test.command", "COMMAND"));
    sdk.registerDescriptor(descriptor("test.other", "COMMAND"));
    const port: CommandCapabilityPort = {
      async command(request: CommandRequest) {
        return { capability_id: request.capability_id, output: { applied: true } };
      },
    };
    sdk.registerCommand("test.command", port);
    sdk.registerCommand("test.other", port);
    const base: CommandRequest = {
      capability_id: "test.command",
      input: {},
      idempotency_key: "op-shared",
      context: ctx(),
    };
    await sdk.command(base);
    await expect(
      sdk.command({ ...base, capability_id: "test.other" }),
    ).rejects.toMatchObject({ code: "CONFLICT" });
  });
});

describe("ep011_unit_sdk_idempotency_tracker", () => {
  it("binds key to capability", () => {
    const tracker = new IdempotencyTracker();
    expect(
      tracker.record({ key: "k", capability_id: "a", result: { ok: true } }),
    ).toBeUndefined();
    expect(tracker.size).toBe(1);
    expect(
      tracker.record({ key: "k", capability_id: "b", result: { ok: true } }),
    ).toMatchObject({ code: "CONFLICT" });
  });
});
