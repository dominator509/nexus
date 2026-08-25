/**
 * EP-042 M2 deterministic compatibility proofs (SPEC-016).
 *
 * Same input -> same decision. Unknown platform denied, unknown current
 * version denied, unsupported target version denied, incompatible
 * component set denied, deterministic ordering. COMPATIBILITY MATRIX
 * EXISTS != UPDATE SAFE.
 */

import { describe, expect, it } from "vitest";
import {
  ReleaseError,
  evaluateCompatibility,
  matrixSupportsAllProfiles,
  matrixSupportsProfile,
  parseReleaseManifest,
} from "@nexus/setup";
import {
  componentWire,
  fixtureManifest,
  manifestWire,
  matrixWire,
} from "./fixtures";

function componentsWithVersions(
  versions: Record<string, string>,
): ReturnType<typeof fixtureManifest>["components"] {
  const wire = {
    ...manifestWire(),
    components: Object.entries(versions).map(([id, version]) =>
      componentWire(id, version),
    ),
    release_id: "release-x",
    version: "x",
  };
  return parseReleaseManifest(wire).components;
}

describe("ep042_unit compatibility", () => {
  it("ep042_unit_compatibility_accepts_in_range_components", () => {
    const manifest = fixtureManifest();
    const verdict = evaluateCompatibility(
      manifest.compatibility,
      manifest.components,
    );
    expect(verdict.compatible).toBe(true);
    expect(verdict.reasons).toHaveLength(0);
  });

  it("ep042_unit_compatibility_is_deterministic", () => {
    const manifest = fixtureManifest();
    const a = evaluateCompatibility(
      manifest.compatibility,
      manifest.components,
    );
    const b = evaluateCompatibility(
      manifest.compatibility,
      manifest.components,
    );
    expect(a).toEqual(b);
  });

  it("ep042_unit_compatibility_rejects_unknown_component", () => {
    const manifest = fixtureManifest();
    const verdict = evaluateCompatibility(manifest.compatibility, [
      ...manifest.components,
      {
        component_id: "comp-99",
        name: "unknown",
        version: "1.0.0",
        artifact_ref: { backend: "local", key: "a" },
        digest: componentWire("comp-99", "1.0.0").digest as string,
        signature: {
          algorithm: "ED25519",
          key_id: "k",
          value_b64: "AAAA01BBBB01",
        },
        sbom_ref: { backend: "local", key: "s" },
        license_ref: "MIT",
        size_bytes: 1,
      },
    ]);
    expect(verdict.compatible).toBe(false);
    expect(verdict.reasons.join(" ")).toContain("not present");
  });

  it("ep042_unit_compatibility_rejects_unknown_current_version", () => {
    const manifest = fixtureManifest();
    const components = componentsWithVersions({ "comp-1": "9.9.9" });
    const verdict = evaluateCompatibility(manifest.compatibility, components);
    expect(verdict.compatible).toBe(false);
    expect(verdict.reasons.join(" ")).toContain("exceeds matrix maximum");
  });

  it("ep042_unit_compatibility_rejects_version_mismatch", () => {
    const manifest = fixtureManifest();
    const components = componentsWithVersions({ "comp-1": "1.5.0" });
    const verdict = evaluateCompatibility(manifest.compatibility, components);
    expect(verdict.compatible).toBe(false);
    expect(verdict.reasons.join(" ")).toContain(
      "does not match matrix version",
    );
  });

  it("ep042_unit_compatibility_rejects_below_minimum", () => {
    const manifest = fixtureManifest();
    const components = componentsWithVersions({ "comp-1": "0.9.0" });
    const verdict = evaluateCompatibility(manifest.compatibility, components);
    expect(verdict.compatible).toBe(false);
  });

  it("ep042_unit_compatibility_rejects_unparseable_version", () => {
    const manifest = fixtureManifest();
    const components = componentsWithVersions({ "comp-1": "latest" });
    const verdict = evaluateCompatibility(manifest.compatibility, components);
    expect(verdict.compatible).toBe(false);
  });

  it("ep042_unit_compatibility_supports_all_profiles", () => {
    const manifest = fixtureManifest();
    expect(matrixSupportsAllProfiles(manifest.compatibility)).toBe(true);
    for (const profile of [
      "MANAGED",
      "BYOC",
      "EXISTING_SSH",
      "HYBRID",
      "FULLY_LOCAL",
    ] as const) {
      expect(matrixSupportsProfile(manifest.compatibility, profile)).toBe(true);
    }
  });

  it("ep042_unit_compatibility_rejects_matrix_with_duplicate_entries", () => {
    const wire = structuredClone(matrixWire());
    const entries = wire["entries"] as Array<Record<string, unknown>>;
    entries.push({ ...(entries[0] ?? {}) });
    // The parser fails closed on duplicate component entries.
    expect(() =>
      parseReleaseManifest({ ...fixtureManifestWire(), compatibility: wire }),
    ).toThrow(ReleaseError);
  });

  it("ep042_unit_compatibility_rejects_empty_matrix", () => {
    const wire = { ...matrixWire(), entries: [] };
    expect(() =>
      parseReleaseManifest({ ...fixtureManifestWire(), compatibility: wire }),
    ).toThrow(ReleaseError);
  });

  it("ep042_unit_compatibility_rejects_unknown_profile_in_entry", () => {
    const wire = structuredClone(matrixWire());
    const entries = wire["entries"] as Array<Record<string, unknown>>;
    entries[0] = { ...(entries[0] ?? {}), supported_profiles: ["CLOUD_ONLY"] };
    expect(() =>
      parseReleaseManifest({ ...fixtureManifestWire(), compatibility: wire }),
    ).toThrow(ReleaseError);
  });

  it("ep042_unit_compatibility_reasons_sorted_deterministically", () => {
    const manifest = fixtureManifest();
    const components = componentsWithVersions({
      "comp-1": "9.9.9",
      "comp-2": "0.1.0",
    });
    const a = evaluateCompatibility(manifest.compatibility, components);
    const reversed = [...components].reverse();
    const b = evaluateCompatibility(manifest.compatibility, reversed);
    expect(a).toEqual(b);
    expect(a.compatible).toBe(false);
  });
});

function fixtureManifestWire(): Record<string, unknown> {
  return {
    schema_version: 1,
    release_id: "release-1",
    version: "1.0.0",
    channel: "STABLE",
    components: [
      componentWire("comp-1", "1.0.0"),
      componentWire("comp-2", "2.0.0"),
    ],
    compatibility: matrixWire(),
    sbom_ref: { backend: "local", key: "sbom-root" },
    license_refs: ["MIT"],
    created_at: "2026-08-25T00:00:00Z",
  };
}
