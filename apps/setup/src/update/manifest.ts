/**
 * EP-042 M2 manifest validation behavior (SPEC-016, SPEC-024).
 *
 * Deterministic, pure, fail-closed. The manifest parser is the only
 * entry point: a raw object can never become a typed ReleaseManifest
 * without passing through deny-unknown validation. Digest binding is
 * real SHA-256 content addressing (strip-then-digest: the digest field
 * itself is excluded so the binding is verifiable).
 *
 * Permanent invariants:
 * - RELEASE MANIFEST EXISTS != RELEASE VERIFIED
 * - DIGEST PRESENT != ARTIFACT VERIFIED
 * - MANIFEST PARSE SUCCESS != RELEASE VERIFIED
 */

import { contentDigest } from "./digest";
import { ReleaseError, ReleaseErrorCode } from "./errors";
import { Digest, parseReleaseManifest } from "./types";
import type { ReleaseManifest, VerificationState } from "./types";

/**
 * Parse and validate a wire-shaped release manifest. Fail-closed on:
 * missing/malformed input, unsupported schema_version, unknown
 * vocabulary, unknown fields, missing component digests, malformed
 * digests, duplicate component identities, missing compatibility
 * matrix, and malformed timestamps.
 */
export function parseManifestWire(
  value: unknown,
  what = "release manifest",
): ReleaseManifest {
  return parseReleaseManifest(value, what);
}

/**
 * Canonical wire object for a manifest WITHOUT its digest field
 * (strip-then-digest). Property order follows the canonical Rust serde
 * field order so serialization is deterministic.
 */
export function manifestCanonicalObject(
  manifest: ReleaseManifest,
): Record<string, unknown> {
  return {
    schema_version: manifest.schema_version,
    release_id: manifest.release_id,
    version: manifest.version,
    channel: manifest.channel,
    components: manifest.components,
    compatibility: manifest.compatibility,
    ...(manifest.offline_bundle_ref !== undefined
      ? { offline_bundle_ref: manifest.offline_bundle_ref }
      : {}),
    sbom_ref: manifest.sbom_ref,
    license_refs: manifest.license_refs,
    created_at: manifest.created_at,
  };
}

/** Real content digest over the canonical manifest bytes. */
export async function manifestContentDigest(
  manifest: ReleaseManifest,
): Promise<Digest> {
  return contentDigest(manifestCanonicalObject(manifest));
}

/**
 * Verify the manifest's declared digest binding. MISSING != VERIFIED:
 * a manifest without a digest binding is simply unverified.
 */
export async function verifyManifestDigestBinding(
  manifest: ReleaseManifest,
): Promise<VerificationState> {
  if (manifest.manifest_digest === undefined) {
    return "MISSING";
  }
  const declared = Digest.parse(manifest.manifest_digest, "manifest_digest");
  const computed = await manifestContentDigest(manifest);
  return computed.equals(declared) ? "VERIFIED" : "MISMATCH";
}

/**
 * RELEASE MANIFEST EXISTS != RELEASE VERIFIED: exposes the verification
 * state ladder; a manifest never self-certifies.
 */
export async function manifestVerificationState(
  manifest: ReleaseManifest,
): Promise<VerificationState> {
  return verifyManifestDigestBinding(manifest);
}

/**
 * Fail-closed duplicate-component check: a release manifest must not
 * declare the same component identity twice. Throws on duplicates.
 */
export function assertNoDuplicateComponents(manifest: ReleaseManifest): void {
  const seen = new Set<string>();
  for (const component of manifest.components) {
    if (seen.has(component.component_id)) {
      throw new ReleaseError(
        ReleaseErrorCode.Validation,
        `release manifest declares duplicate component: ${component.component_id}`,
        { field: "components" },
      );
    }
    seen.add(component.component_id);
  }
}

/**
 * Signature honesty: a component signature field existing is PRESENT,
 * never VALID. No cryptographic verifier exists in this surface; the
 * state ladder is exposed so callers cannot collapse presence into
 * validity.
 */
export function componentSignatureState(_signature: {
  key_id: string;
  value_b64: string;
}): "PRESENT" {
  return "PRESENT";
}

/**
 * Fail-closed manifest acceptance used by the planner: the manifest must
 * parse, carry at least one component, a compatibility matrix, licenses,
 * and a verified digest binding when one is declared. Throws on any
 * unmet precondition.
 */
export async function assertManifestAcceptable(
  manifest: ReleaseManifest,
): Promise<void> {
  assertNoDuplicateComponents(manifest);
  if (manifest.components.length === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      "release manifest must contain at least one signed component",
      { field: "components" },
    );
  }
  if (manifest.compatibility.entries.length === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      "release manifest must contain a non-empty compatibility matrix",
      { field: "compatibility" },
    );
  }
  if (manifest.license_refs.length === 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      "release manifest must reference at least one license",
      { field: "license_refs" },
    );
  }
  const binding = await verifyManifestDigestBinding(manifest);
  if (binding === "MISMATCH") {
    throw new ReleaseError(
      ReleaseErrorCode.DigestMismatch,
      "release manifest digest binding does not match its content",
      { field: "manifest_digest" },
    );
  }
}
