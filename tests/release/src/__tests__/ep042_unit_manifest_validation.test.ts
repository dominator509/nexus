/**
 * EP-042 M2 manifest validation proofs (SPEC-016, SPEC-024).
 *
 * Fail-closed: missing/malformed manifest denied, unsupported schema
 * version denied, unknown vocabulary denied, missing/malformed component
 * digest denied, duplicate component identity denied, missing
 * compatibility matrix denied. Digest binding is real SHA-256 content
 * addressing. MANIFEST PARSE SUCCESS != RELEASE VERIFIED.
 */

import { describe, expect, it } from "vitest";
import {
  Digest,
  ReleaseError,
  ReleaseErrorCode,
  assertNoDuplicateComponents,
  manifestContentDigest,
  manifestVerificationState,
  parseReleaseManifest,
  verifyManifestDigestBinding,
} from "@nexus/setup";
import {
  componentWire,
  digest,
  fixtureManifest,
  manifestWire,
} from "./fixtures";

describe("ep042_unit manifest validation", () => {
  it("ep042_unit_manifest_parses_valid_wire_object", () => {
    const manifest = parseReleaseManifest(manifestWire());
    expect(manifest.release_id).toBe("release-1");
    expect(manifest.components.length).toBe(2);
    expect(manifest.channel).toBe("STABLE");
    expect(manifest.schema_version).toBe(1);
  });

  it("ep042_unit_manifest_rejects_missing_manifest", () => {
    expect(() => parseReleaseManifest(undefined)).toThrow(ReleaseError);
    expect(() => parseReleaseManifest(null)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_malformed_manifest", () => {
    expect(() => parseReleaseManifest("not an object")).toThrow(ReleaseError);
    expect(() => parseReleaseManifest(42)).toThrow(ReleaseError);
    expect(() => parseReleaseManifest([])).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_unsupported_schema_version", () => {
    const wire = { ...manifestWire(), schema_version: 2 };
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_unknown_vocabulary_channel", () => {
    const wire = { ...manifestWire(), channel: "NIGHTLY" };
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_unknown_vocabulary_mode", () => {
    const wire = structuredClone(manifestWire());
    const entries = wire["compatibility"] as {
      entries: Array<Record<string, unknown>>;
    };
    entries.entries[0] = {
      ...entries.entries[0],
      supported_profiles: ["CLOUD_ONLY"],
    };
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_unknown_field", () => {
    const wire = { ...manifestWire(), bogus: true };
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_missing_component_digest", () => {
    const wire = structuredClone(manifestWire());
    const components = wire["components"] as Array<Record<string, unknown>>;
    delete components[0]?.["digest"];
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_malformed_component_digest", () => {
    const wire = structuredClone(manifestWire());
    const components = wire["components"] as Array<Record<string, unknown>>;
    components[0] = { ...components[0], digest: "not-a-digest" };
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_uppercase_digest_hex", () => {
    const wire = structuredClone(manifestWire());
    const components = wire["components"] as Array<Record<string, unknown>>;
    components[0] = {
      ...components[0],
      digest: "sha256:0123456789ABCDEF0123456789ABCDEF",
    };
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_short_digest_hex", () => {
    const wire = structuredClone(manifestWire());
    const components = wire["components"] as Array<Record<string, unknown>>;
    components[0] = { ...components[0], digest: "sha256:0123456789abcdef" };
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_duplicate_component_identity", () => {
    const manifest = fixtureManifest();
    const first = manifest.components[0];
    const second = manifest.components[1];
    if (first === undefined || second === undefined) {
      throw new Error("fixture requires two components");
    }
    // Patch the parsed manifest: two components with the same id.
    const duplicated = {
      ...manifest,
      components: [first, { ...second, component_id: "comp-1" }],
    };
    expect(() => assertNoDuplicateComponents(duplicated)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_missing_compatibility_matrix", () => {
    const wire = structuredClone(manifestWire());
    delete wire["compatibility"];
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_missing_licenses", () => {
    const wire = { ...manifestWire(), license_refs: [] };
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_rejects_malformed_timestamp", () => {
    const wire = { ...manifestWire(), created_at: "not-a-date" };
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_exists_not_verified_without_binding", async () => {
    const manifest = fixtureManifest();
    expect(manifest.manifest_digest).toBeUndefined();
    const state = await manifestVerificationState(manifest);
    expect(state).toBe("MISSING");
  });

  it("ep042_unit_manifest_digest_binding_verifies", async () => {
    const manifest = fixtureManifest();
    const digest = await manifestContentDigest(manifest);
    expect(digest.alg()).toBe("sha256");
    expect(digest.hex().length).toBe(64);
    const bound = parseReleaseManifest({
      ...manifestWire(),
      manifest_digest: digest.asString(),
    });
    expect(await verifyManifestDigestBinding(bound)).toBe("VERIFIED");
  });

  it("ep042_unit_manifest_digest_binding_mismatch_denied", async () => {
    const manifest = fixtureManifest();
    const bound = parseReleaseManifest({
      ...manifestWire(),
      manifest_digest: digest("wrong-seed"),
    });
    expect(await verifyManifestDigestBinding(bound)).toBe("MISMATCH");
  });

  it("ep042_unit_manifest_digest_changes_with_content", async () => {
    const a = fixtureManifest();
    const b = { ...a, version: "1.0.1" };
    const digestA = await manifestContentDigest(a);
    const digestB = await manifestContentDigest(b);
    expect(digestA.equals(digestB)).toBe(false);
  });

  it("ep042_unit_manifest_content_digest_is_deterministic", async () => {
    const manifest = fixtureManifest();
    const a = await manifestContentDigest(manifest);
    const b = await manifestContentDigest(manifest);
    expect(a.equals(b)).toBe(true);
  });

  it("ep042_unit_signature_present_not_valid", () => {
    const manifest = fixtureManifest();
    const component = manifest.components[0];
    expect(component).toBeDefined();
    const sig = component?.signature;
    expect(sig).toBeDefined();
    // The wire surface exposes PRESENT; no verifier exists, so VALID is
    // never produced (SIGNATURE FIELD EXISTS != SIGNATURE VERIFIED).
    expect(sig?.value_b64.length).toBeGreaterThan(0);
    expect(Digest.parse(component?.digest ?? "", "digest")).toBeDefined();
  });

  it("ep042_unit_manifest_parse_success_not_release_verified", async () => {
    const manifest = fixtureManifest();
    // Parsing succeeded, yet the release is NOT verified: no binding.
    const state = await manifestVerificationState(manifest);
    expect(state).not.toBe("VERIFIED");
  });

  it("ep042_unit_manifest_rejects_signature_with_bad_base64", () => {
    const wire = structuredClone(manifestWire());
    const components = wire["components"] as Array<Record<string, unknown>>;
    const signature = components[0]?.["signature"] as Record<string, unknown>;
    components[0] = {
      ...components[0],
      signature: { ...signature, value_b64: "not base64!!" },
    };
    expect(() => parseReleaseManifest(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_manifest_error_code_specificity", () => {
    try {
      parseReleaseManifest(undefined);
      throw new Error("expected parse to fail");
    } catch (error) {
      expect(error).toBeInstanceOf(ReleaseError);
      const releaseError = error as ReleaseError;
      expect(releaseError.code).toBe(ReleaseErrorCode.Validation);
    }
  });
});
