import { describe, expect, it } from "vitest";

import { readFileSync } from "node:fs";
import path from "node:path";

import { WorkflowContractError } from "../errors.js";
import {
  ACTIVITY_CONTRACT_VERSION,
  QUERY_SCHEMA_VERSION,
  SIGNAL_SCHEMA_VERSION,
  WORKFLOW_COMPATIBILITY,
  WORKFLOW_CONTRACT_VERSION,
  compatibilityFor,
  isCompatibleSchemaVersion,
  isCompatibleSignalVersion,
  parseSemver,
} from "../versioning.js";
import { workflowsPackageRoot } from "./helpers/fixtures.js";

describe("ep006_unit_versioning", () => {
  it("ep006_unit_versioning_constants_parse", () => {
    expect(parseSemver(WORKFLOW_CONTRACT_VERSION)).toEqual({
      major: 1,
      minor: 0,
      patch: 0,
    });
    expect(parseSemver(SIGNAL_SCHEMA_VERSION).major).toBe(1);
    expect(parseSemver(QUERY_SCHEMA_VERSION).major).toBe(1);
    expect(parseSemver(ACTIVITY_CONTRACT_VERSION).major).toBe(1);
  });

  it("ep006_unit_versioning_rejects_bad_semver", () => {
    expect(() => parseSemver("abc")).toThrow(WorkflowContractError);
    expect(() => parseSemver("1.0")).toThrow(WorkflowContractError);
    expect(() => parseSemver("1.0.0.0")).toThrow(WorkflowContractError);
  });

  it("ep006_unit_versioning_compatibility_matrix_consistent", () => {
    // Every workflow kind must declare version compatibility.
    for (const kind of [
      "OBJECTIVE",
      "APPROVAL",
      "CONNECTOR_CERTIFICATION",
      "INCIDENT_REMEDIATION",
      "DEPLOYMENT",
    ] as const) {
      const entry = compatibilityFor(kind);
      expect(entry.workflowKind).toBe(kind);
      expect(parseSemver(entry.minSupportedSignalVersion).major).toBe(1);
      expect(parseSemver(entry.minSupportedQueryVersion).major).toBe(1);
      expect(isCompatibleSignalVersion(SIGNAL_SCHEMA_VERSION, entry)).toBe(
        true,
      );
      expect(isCompatibleSignalVersion(QUERY_SCHEMA_VERSION, entry)).toBe(true);
    }
    expect(WORKFLOW_COMPATIBILITY.length).toBe(5);
  });

  it("ep006_unit_versioning_same_major_compatible", () => {
    expect(isCompatibleSchemaVersion("1.2.0", "1.0.0")).toBe(true);
    expect(isCompatibleSchemaVersion("1.0.0", "1.0.0")).toBe(true);
  });

  it("ep006_unit_versioning_rejects_major_mismatch_fail_closed", () => {
    expect(isCompatibleSchemaVersion("2.0.0", "1.0.0")).toBe(false);
    expect(isCompatibleSchemaVersion("0.9.0", "1.0.0")).toBe(false);
  });

  it("ep006_unit_versioning_signal_below_min_rejected", () => {
    const entry = compatibilityFor("APPROVAL");
    expect(isCompatibleSignalVersion("0.9.0", entry)).toBe(false);
  });

  it("ep006_unit_versioning_signal_newer_minor_same_major_accepted", () => {
    const entry = compatibilityFor("APPROVAL");
    expect(isCompatibleSignalVersion("1.3.0", entry)).toBe(true);
  });

  it("ep006_unit_versioning_doc_has_strategy_markers", () => {
    // SPEC-023 behavior 8 requires a documented in-flight compatibility
    // strategy; the strategy doc must cover the mandated mechanisms.
    const doc = readFileSync(
      path.join(workflowsPackageRoot, "docs/versioning.md"),
      "utf8",
    );
    for (const marker of [
      "patched",
      "version()",
      "task queue",
      "compatible set",
      "in-flight",
      "replay",
      "idempotency",
    ]) {
      expect(doc).toContain(marker);
    }
  });

  it("ep006_unit_versioning_doc_covers_breaking_change_rule", () => {
    const doc = readFileSync(
      path.join(workflowsPackageRoot, "docs/versioning.md"),
      "utf8",
    );
    expect(doc).toContain("Major (breaking)");
    expect(doc).toContain("ADR");
  });
});
