/**
 * EP-035 M1 canonical schema parity tests (anti-drift).
 *
 * The setup contracts bind to canonical schema vocabulary. This test
 * reads the authoritative schema files and asserts field-name and enum
 * parity, so a schema change the setup contract did not follow fails
 * the milestone.
 */

import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const SCHEMAS = join(process.cwd(), "..", "..", "schemas");

function loadSchema(rel: string): {
  properties: Record<string, unknown>;
  required?: Array<string>;
} {
  const raw = readFileSync(join(SCHEMAS, rel), "utf8");
  return JSON.parse(raw) as {
    properties: Record<string, unknown>;
    required?: Array<string>;
  };
}

describe("ep035_unit_schema_parity", () => {
  it("deployment profile contract matches the canonical schema fields", () => {
    const schema = loadSchema("deployment-profile.schema.json");
    const expected = Object.keys(schema.properties).sort();
    expect(expected).toEqual(
      [
        "backup",
        "components",
        "id",
        "mode",
        "nodes",
        "release_channel",
        "remote_access",
      ].sort(),
    );
    expect(schema.required).toEqual([
      "id",
      "mode",
      "release_channel",
      "components",
      "nodes",
      "backup",
      "remote_access",
    ]);
  });

  it("deployment mode enum matches the canonical schema", () => {
    const schema = loadSchema("deployment-profile.schema.json");
    const mode = schema.properties["mode"] as { enum?: Array<string> };
    expect(mode.enum).toEqual([
      "MANAGED",
      "BYOC",
      "EXISTING_SSH",
      "HYBRID",
      "FULLY_LOCAL",
    ]);
  });

  it("release channel enum matches the canonical schema", () => {
    const schema = loadSchema("deployment-profile.schema.json");
    const channel = schema.properties["release_channel"] as {
      enum?: Array<string>;
    };
    expect(channel.enum).toEqual(["STABLE", "BETA", "DEVELOPER", "PINNED"]);
  });

  it("recovery kit contract matches the canonical recovery-kit schema fields", () => {
    const schema = loadSchema("auth/recovery-kit.schema.json");
    const expected = Object.keys(schema.properties).sort();
    expect(expected).toEqual(
      [
        "kit_id",
        "principal_id",
        "tenant_id",
        "material_kind",
        "created_at_unix_s",
        "expires_at_unix_s",
        "correlation",
      ].sort(),
    );
    expect(schema.required).toEqual([
      "kit_id",
      "principal_id",
      "tenant_id",
      "material_kind",
      "created_at_unix_s",
      "expires_at_unix_s",
      "correlation",
    ]);
  });

  it("recovery material kind enum matches the canonical schema", () => {
    const schema = loadSchema("auth/recovery-kit.schema.json");
    const kind = schema.properties["material_kind"] as {
      enum?: Array<string>;
    };
    expect(kind.enum).toEqual([
      "RECOVERY_CODES",
      "OFFLINE_PASSPHRASE",
      "DEVICE_BACKUP",
    ]);
  });
});
