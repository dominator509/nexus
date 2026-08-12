import { describe, expect, it } from "vitest";
import {
  type ActionRequest,
  type NexusControlObject,
  type JsonValue,
} from "../generated.js";

describe("generated contracts", () => {
  it("exports the canonical control object shape", () => {
    const obj: NexusControlObject = {
      schema_version: "1.0.0",
      intent: "home.lights.set",
      route: "DETERMINISTIC",
      risk: "R0",
      privacy: "HOUSEHOLD",
      ambiguity: 0,
      approval_required: false,
      executable_instruction: true,
      confidence: 0.99,
      required_capabilities: ["home.lights.set"],
      entities: {},
    };
    expect(obj.intent).toBe("home.lights.set");
    expect(obj.required_capabilities).toHaveLength(1);
    expect(obj.schema_version).toBe("1.0.0");
  });

  it("accepts an action request with canonical fields", () => {
    const req: ActionRequest = {
      action_id: "act_1",
      tenant_id: "tenant_1",
      principal_id: "user_1",
      capability_id: "cap.lock",
      idempotency_key: "key_1",
      risk: "R3",
      approval_class: "HUMAN",
      reversal: "COMPENSATING",
      arguments: { door: "front" },
      expected_state: { locked: true },
      invocation: {
        request_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073",
        correlation_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6074",
        origin_system: "voice",
        external_actor_id: "user_1",
        external_actor_type: "PERSON",
        channel: "voice",
      },
    };
    expect(req.idempotency_key).toBe("key_1");
    expect(req.approval_class).toBe("HUMAN");
    expect(req.invocation.request_id).toMatch(/^0190/);
  });

  it("JsonValue covers nested structures", () => {
    const v: JsonValue = { a: [1, "two", null, { b: true }] };
    expect(Array.isArray(v.a)).toBe(true);
  });
});
