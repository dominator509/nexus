/**
 * EP-033 M1 schema parity test (anti-drift).
 *
 * The dashboard contracts bind to canonical schema vocabulary. This
 * test reads the authoritative schema files and asserts field-name and
 * enum parity, so a schema change that the UI contract did not follow
 * fails the milestone.
 */

import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const SCHEMAS = join(process.cwd(), "..", "..", "schemas");

function loadSchema(rel: string): { properties: Record<string, unknown>; required?: Array<string> } {
  const raw = readFileSync(join(SCHEMAS, rel), "utf8");
  return JSON.parse(raw) as { properties: Record<string, unknown>; required?: Array<string> };
}

describe("ep033_unit_schema_parity", () => {
  it("session contract matches the canonical auth-session schema fields", () => {
    const schema = loadSchema("auth/auth-session.schema.json");
    const expected = Object.keys(schema.properties).sort();
    expect(expected).toEqual(
      [
        "session_id",
        "principal_id",
        "tenant_id",
        "device_id",
        "grant_flow",
        "strength",
        "created_at_unix_s",
        "expires_at_unix_s",
        "revoked",
        "correlation",
      ].sort(),
    );
    expect(schema.required).toContain("session_id");
    expect(schema.required).toContain("correlation");
  });

  it("grant_flow enum matches the canonical auth-session schema", () => {
    const schema = loadSchema("auth/auth-session.schema.json");
    const grantFlow = schema.properties["grant_flow"] as { enum?: Array<string> };
    expect(grantFlow.enum).toEqual(["AUTHORIZATION_CODE", "REFRESH_TOKEN", "CLIENT_CREDENTIALS"]);
  });

  it("strength enum matches the canonical auth-session schema", () => {
    const schema = loadSchema("auth/auth-session.schema.json");
    const strength = schema.properties["strength"] as { enum?: Array<string> };
    expect(strength.enum).toEqual(["NONE", "SINGLE_FACTOR", "MULTI_FACTOR", "STEP_UP"]);
  });

  it("command contract matches the canonical action-request schema fields", () => {
    const schema = loadSchema("action-request.schema.json");
    const expected = Object.keys(schema.properties).sort();
    expect(expected).toEqual(
      [
        "action_id",
        "tenant_id",
        "principal_id",
        "capability_id",
        "idempotency_key",
        "risk",
        "approval_class",
        "reversal",
        "arguments",
        "expected_state",
        "invocation",
      ].sort(),
    );
    expect(schema.required).toContain("capability_id");
    expect(schema.required).toContain("idempotency_key");
  });

  it("risk enum matches the canonical action-request schema", () => {
    const schema = loadSchema("action-request.schema.json");
    const risk = schema.properties["risk"] as { enum?: Array<string> };
    expect(risk.enum).toEqual(["R0", "R1", "R2", "R3", "R4"]);
  });

  it("capability approval enum matches the canonical capability-descriptor schema", () => {
    const schema = loadSchema("capability-descriptor.schema.json");
    const approval = schema.properties["approval"] as { enum?: Array<string> };
    expect(approval.enum).toEqual(["NONE", "POLICY", "HUMAN", "STRONG_HUMAN", "FOUR_EYES"]);
  });

  it("capability class enum matches the canonical capability-descriptor schema", () => {
    const schema = loadSchema("capability-descriptor.schema.json");
    const klass = schema.properties["class"] as { enum?: Array<string> };
    expect(klass.enum).toEqual(["QUERY", "COMMAND", "WORKFLOW", "STREAM", "ADMINISTRATIVE"]);
  });

  it("capability availability enum matches the canonical capability-descriptor schema", () => {
    const schema = loadSchema("capability-descriptor.schema.json");
    const availability = schema.properties["availability"] as { enum?: Array<string> };
    expect(availability.enum).toEqual(["AVAILABLE", "DEGRADED", "UNAVAILABLE", "UNCERTIFIED"]);
  });

  it("business context contract matches the canonical hydra schema fields", () => {
    const schema = loadSchema("hydra/business-context.schema.json");
    const expected = Object.keys(schema.properties).sort();
    expect(expected).toEqual(["tenant_id", "principal_id", "scope", "business_id", "correlation"].sort());
    expect(schema.required).toEqual(["tenant_id", "principal_id", "scope"]);
  });

  it("event subscription binds the canonical event-envelope vocabulary", () => {
    const schema = loadSchema("event-envelope.schema.json");
    const expected = Object.keys(schema.properties).sort();
    expect(expected).toContain("event_type");
    expect(expected).toContain("source");
    expect(expected).toContain("schema_version");
    expect(expected).toContain("correlation_id");
    expect(expected).toContain("tenant_id");
    expect(expected).toContain("data_class");
    expect(schema.required).toContain("payload");
  });
});
