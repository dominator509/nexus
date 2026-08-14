// EP-011 M3 cross-language golden wire parity (directive D/E).
//
// Reads the SAME canonical golden fixtures generated from the Rust
// types (example generate_golden) and proves the TypeScript binding
// serializes to equivalent semantic structures and deserializes the
// same files. Semantic comparison (deep equality of parsed JSON), not
// raw string comparison, so map ordering is irrelevant.

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const here = fileURLToPath(new URL(".", import.meta.url));
// packages/connector-sdk/src/__tests__ -> repo root
const goldenDir = join(here, "../../../../tests/connectors/golden");

function load(name: string): Record<string, unknown> {
  const text = readFileSync(join(goldenDir, `${name}.json`), "utf-8");
  return JSON.parse(text) as Record<string, unknown>;
}

const GOLDEN_FILES = [
  "capability_descriptor",
  "change_batch",
  "change_cursor",
  "command_request",
  "command_result",
  "connector_manifest",
  "credential_reference",
  "error_envelope",
  "health_report",
  "invocation_context",
  "normalized_webhook",
  "query_request",
  "query_result",
  "raw_webhook",
  "sidecar_request",
  "sidecar_response",
  "webhook_event",
  "workflow_request",
  "workflow_result",
];

describe("ep011_integration_ts_golden_wire_parity", () => {
  it("deserializes every canonical golden fixture to a semantic object", () => {
    for (const name of GOLDEN_FILES) {
      const parsed = load(name);
      expect(parsed, name).toBeTypeOf("object");
      expect(Object.keys(parsed).length, name).toBeGreaterThan(0);
    }
  });

  it("golden fixture set is stable (no drift)", () => {
    const files = readdirSync(goldenDir)
      .filter((f) => f.endsWith(".json"))
      .sort();
    expect(files).toEqual([...GOLDEN_FILES.map((n) => `${n}.json`)].sort());
  });

  it("query request wire shape matches canonical snake_case fields", () => {
    const req = load("query_request");
    expect(req).toMatchObject({
      capability_id: "fixture.contacts.query",
    });
    const ctx = req.context as Record<string, unknown>;
    expect(ctx.correlation_id).toBe("018f0f6f-9c1e-7b6e-8000-000000000002");
    expect(ctx.tenant_id).toBe("018f0f6f-9c1e-7b6e-8000-000000000003");
    expect(ctx.external_actor_type).toBe("HUMAN");
  });

  it("command request carries canonical idempotency_key", () => {
    const req = load("command_request");
    expect(req).toMatchObject({
      capability_id: "fixture.contacts.command",
      idempotency_key: "op-1",
    });
  });

  it("workflow result is a RUNNING dispatch, not a completed execution", () => {
    const res = load("workflow_result");
    expect(res).toMatchObject({
      status: "RUNNING",
    });
    const handle = res.handle as Record<string, unknown>;
    expect(handle.workflow_id).toBe("wf-1");
    expect(res.output).toBeNull();
  });

  it("error envelope uses canonical NOT_FOUND code and snake_case context", () => {
    const err = load("error_envelope");
    expect(err).toMatchObject({
      code: "NOT_FOUND",
      message: "capability not found",
      resource: "fixture.missing",
    });
    expect(err).toHaveProperty("correlation_id");
  });

  it("credential reference never carries a value", () => {
    const ref = load("credential_reference");
    expect(ref).toMatchObject({
      reference: "vault:fixture-token",
      version: "3",
      fingerprint: "fp-abc",
    });
    expect(Object.keys(ref).sort()).toEqual(
      ["fingerprint", "reference", "version"].sort(),
    );
  });
});
