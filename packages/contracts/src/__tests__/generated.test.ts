import { describe, expect, it } from "vitest";
import {
  type ActionRequest,
  type NexusControlObject,
  type JsonValue,
} from "../generated.js";

describe("generated contracts", () => {
  it("exports the canonical control object shape", () => {
    const obj: NexusControlObject = {
      schemaVersion: "1",
      intent: "home.lights.set",
      route: "DETERMINISTIC",
      risk: "R0",
      privacy: "HOUSEHOLD",
      ambiguity: 0,
      approvalRequired: false,
      executableInstruction: true,
      confidence: 0.99,
      requiredCapabilities: ["home.lights.set"],
      entities: {},
    };
    expect(obj.intent).toBe("home.lights.set");
    expect(obj.requiredCapabilities).toHaveLength(1);
  });

  it("accepts an action request with canonical fields", () => {
    const req: ActionRequest = {
      actionId: "act_1",
      tenantId: "tenant_1",
      principalId: "user_1",
      capabilityId: "cap.lock",
      idempotencyKey: "key_1",
      risk: "R3",
      approvalClass: "HUMAN",
      reversal: "COMPENSATING",
      arguments: { door: "front" },
      expectedState: { locked: true },
      invocation: { channel: "voice" },
    };
    expect(req.idempotencyKey).toBe("key_1");
    expect(req.approvalClass).toBe("HUMAN");
  });

  it("JsonValue covers nested structures", () => {
    const v: JsonValue = { a: [1, "two", null, { b: true }] };
    expect(Array.isArray(v.a)).toBe(true);
  });
});
