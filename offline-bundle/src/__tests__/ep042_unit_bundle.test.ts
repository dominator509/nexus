/**
 * EP-042 M5 offline-bundle unit proofs (SPEC-016 behavior 5).
 *
 * Fast unit-level proofs for the bundle model, typed failure
 * classification, and digest binding helpers that do not need the full
 * bundle fixture machinery (the real bundle journey lives in the
 * ep042_bundle_* suite under tests/release).
 */

import { describe, expect, it } from "vitest";
import {
  BUNDLE_FAILURE_CLASSES,
  BUNDLE_KIND_DIRS,
  BUNDLE_KINDS,
  BUNDLE_REQUIRED_KINDS,
  BUNDLE_SCHEMA_VERSION,
  isBundleError,
  isBundleFailureClass,
  BundleError,
} from "../index";

describe("ep042_unit_bundle vocabulary", () => {
  it("ep042_unit_bundle_schema_version_is_one", () => {
    expect(BUNDLE_SCHEMA_VERSION).toBe(1);
  });

  it("ep042_unit_bundle_kinds_match_adr028", () => {
    expect(BUNDLE_KINDS).toEqual([
      "IMAGE",
      "MODEL",
      "LICENSE",
      "SBOM",
      "MIGRATION",
      "RECOVERY_TOOL",
    ]);
    expect(BUNDLE_REQUIRED_KINDS).toEqual([
      "IMAGE",
      "MODEL",
      "LICENSE",
      "SBOM",
    ]);
  });

  it("ep042_unit_bundle_kind_dirs_unique_and_relative", () => {
    const dirs = Object.values(BUNDLE_KIND_DIRS);
    expect(new Set(dirs).size).toBe(dirs.length);
    for (const dir of dirs) {
      expect(dir).not.toMatch(/^[/\\]/);
      expect(dir).not.toMatch(/\.\./);
    }
  });

  it("ep042_unit_bundle_failure_classes_typed", () => {
    expect(BUNDLE_FAILURE_CLASSES.length).toBeGreaterThanOrEqual(13);
    for (const code of BUNDLE_FAILURE_CLASSES) {
      expect(isBundleFailureClass(code)).toBe(true);
      expect(code).toMatch(/^[A-Z_]+$/);
    }
    expect(isBundleFailureClass("NOT_A_CLASS")).toBe(false);
  });

  it("ep042_unit_bundle_error_carries_typed_class", () => {
    const error = new BundleError("PATH_ESCAPE", "escape denied");
    expect(isBundleError(error)).toBe(true);
    expect(error.code).toBe("PATH_ESCAPE");
    const shape = error.toShape();
    expect(shape.code).toBe("PATH_ESCAPE");
    expect(shape.message).toContain("escape denied");
    expect(shape.field).toBeUndefined();
    expect(shape.context).toBeUndefined();
  });

  it("ep042_unit_bundle_error_denies_unknown_class", () => {
    // The class set is closed: a string outside the vocabulary is not a
    // BundleFailureClass (fail-closed classification).
    expect(isBundleFailureClass("SUCCESS")).toBe(false);
  });

  it("ep042_unit_bundle_required_kinds_subset_of_kinds", () => {
    for (const kind of BUNDLE_REQUIRED_KINDS) {
      expect(BUNDLE_KINDS).toContain(kind);
    }
  });
});
