/**
 * EP-043 M2 release manifest production (SPEC-008, SPEC-016).
 *
 * Pure domain: build the canonical release manifest JSON following the
 * EP-042 M1 ReleaseManifest contract shape (schema_version 1,
 * release_id, version, channel, components with real sha256 digests,
 * compatibility, sbom_ref, license_refs, created_at, manifest_digest).
 *
 * Digest discipline: component digests are REAL sha256 over actual
 * artifact bytes; the manifest digest is computed strip-then-digest
 * (self-referential manifest_digest field excluded), matching the
 * canonical EP-042 contract. Signatures are recorded honestly:
 * SIGNATURE_PRESENT_NOT_VERIFIED when no key store/verifier exists.
 */

import { ShipError } from "./errors.ts";
import { canonicalize, sha256Bytes, sha256Hex } from "./model.ts";

/** Canonical release channel vocabulary (mirrors EP-042 M1). */
export const RELEASE_CHANNELS = [
  "STABLE",
  "BETA",
  "DEVELOPER",
  "PINNED",
] as const;
export type ReleaseChannel = (typeof RELEASE_CHANNELS)[number];

/** Canonical deployment profile vocabulary (mirrors deployment-profile schema). */
export const DEPLOYMENT_PROFILE_MODES = [
  "MANAGED",
  "BYOC",
  "EXISTING_SSH",
  "HYBRID",
  "FULLY_LOCAL",
] as const;
export type DeploymentProfileMode = (typeof DEPLOYMENT_PROFILE_MODES)[number];

export interface ObjectRef {
  backend: string;
  key: string;
}

export interface SignedComponentWire {
  component_id: string;
  name: string;
  version: string;
  artifact_ref: ObjectRef;
  digest: string;
  signature: {
    algorithm: string;
    key_id: string;
    value_b64: string;
  };
  sbom_ref: ObjectRef;
  license_ref: string;
  size_bytes: number;
}

export interface ReleaseManifestWire {
  schema_version: 1;
  release_id: string;
  version: string;
  release_channel: ReleaseChannel;
  deployment_profile: DeploymentProfileMode;
  manifest_digest: string;
  components: SignedComponentWire[];
  created_at: string;
}

export interface ManifestComponentInput {
  componentId: string;
  name: string;
  version: string;
  artifactBytes: Uint8Array;
  artifactKey: string;
  backend?: string;
  sbomKey?: string;
  licenseRef?: string;
}

export interface ManifestOptions {
  releaseId: string;
  version: string;
  channel: ReleaseChannel;
  profile: DeploymentProfileMode;
  createdAt: string;
  components: ManifestComponentInput[];
  /** Signature values; when absent the honest PRESENT_NOT_VERIFIED marker is used. */
  signature?: { algorithm: string; keyId: string; valueB64: string };
}

/** Normalize a camelCase signature option to the wire snake_case shape. */
function signatureToWire(
  signature: { algorithm: string; keyId: string; valueB64: string } | undefined,
): { algorithm: string; key_id: string; value_b64: string } {
  if (!signature) {
    return {
      algorithm: "ED25519",
      key_id: "SIGNATURE_PRESENT_NOT_VERIFIED",
      value_b64: "SIGNATURE_PRESENT_NOT_VERIFIED",
    };
  }
  return {
    algorithm: signature.algorithm,
    key_id: signature.keyId,
    value_b64: signature.valueB64,
  };
}

/** Real sha256 digest (alg:hex) over RAW artifact bytes (AUD-077). The
 *  bytes are hashed directly - never decoded through TextDecoder first.
 *  Lossy UTF-8 decoding collapses distinct binary sequences onto the
 *  same replacement characters and would break the artifact binding. */
export function digestBytes(bytes: Uint8Array): string {
  return `sha256:${sha256Bytes(bytes)}`;
}

function assertKnownChannel(value: unknown): ReleaseChannel {
  if (
    typeof value !== "string" ||
    !(RELEASE_CHANNELS as readonly string[]).includes(value)
  ) {
    throw new ShipError(
      "VALIDATION_FAILED",
      `unknown release channel: ${String(value)}`,
    );
  }
  return value as ReleaseChannel;
}

function assertKnownProfile(value: unknown): DeploymentProfileMode {
  if (
    typeof value !== "string" ||
    !(DEPLOYMENT_PROFILE_MODES as readonly string[]).includes(value)
  ) {
    throw new ShipError(
      "VALIDATION_FAILED",
      `unknown deployment profile: ${String(value)}`,
    );
  }
  return value as DeploymentProfileMode;
}

/** Strip-then-digest: canonical payload excludes the self-referential digest. */
export function canonicalManifestPayload(
  manifest: Omit<ReleaseManifestWire, "manifest_digest">,
): string {
  return JSON.stringify(canonicalize(manifest));
}

export function manifestDigest(
  payload: Omit<ReleaseManifestWire, "manifest_digest">,
): string {
  return `sha256:${sha256Hex(canonicalManifestPayload(payload))}`;
}

/** Build the canonical release manifest wire object with real digests. */
export function buildReleaseManifest(
  options: ManifestOptions,
): ReleaseManifestWire {
  if (options.releaseId.length === 0 || options.version.length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "release_id and version must be non-empty",
    );
  }
  if (options.components.length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "release manifest requires at least one component",
    );
  }
  const channel = assertKnownChannel(options.channel);
  const profile = assertKnownProfile(options.profile);

  const components: SignedComponentWire[] = options.components.map(
    (component) => {
      if (
        component.componentId.length === 0 ||
        component.name.length === 0 ||
        component.version.length === 0
      ) {
        throw new ShipError(
          "VALIDATION_FAILED",
          "component id/name/version must be non-empty",
        );
      }
      if (component.artifactBytes.length === 0) {
        throw new ShipError(
          "VALIDATION_FAILED",
          `component artifact must not be empty: ${component.componentId}`,
        );
      }
      const digest = digestBytes(component.artifactBytes);
      const backend = component.backend ?? "local";
      const artifactKey =
        component.artifactKey.length > 0
          ? component.artifactKey
          : `releases/${options.releaseId}/components/${component.componentId}`;
      const sbomKey =
        component.sbomKey ??
        `releases/${options.releaseId}/sbom/${component.componentId}.sbom`;
      const signature = signatureToWire(options.signature);
      return {
        component_id: component.componentId,
        name: component.name,
        version: component.version,
        artifact_ref: { backend, key: artifactKey },
        digest,
        signature,
        sbom_ref: { backend, key: sbomKey },
        license_ref: component.licenseRef ?? "Apache-2.0",
        size_bytes: component.artifactBytes.length,
      };
    },
  );

  const payload: Omit<ReleaseManifestWire, "manifest_digest"> = {
    schema_version: 1,
    release_id: options.releaseId,
    version: options.version,
    release_channel: channel,
    deployment_profile: profile,
    components,
    created_at: options.createdAt,
  };
  return { ...payload, manifest_digest: manifestDigest(payload) };
}

/** Recompute the manifest digest over a wire object (tamper check). */
export function verifyManifestDigest(manifest: ReleaseManifestWire): boolean {
  const { manifest_digest, ...payload } = manifest;
  return manifestDigest(payload) === manifest_digest;
}

/** Parse a manifest wire object fail-closed (deny unknown fields). */
export function parseReleaseManifestWire(value: unknown): ReleaseManifestWire {
  if (typeof value !== "object" || value === null) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "release manifest must be an object",
    );
  }
  const obj = value as Record<string, unknown>;
  if (obj["schema_version"] !== 1) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "unsupported release manifest schema_version",
    );
  }
  const allowed = [
    "schema_version",
    "release_id",
    "version",
    "release_channel",
    "deployment_profile",
    "manifest_digest",
    "components",
    "created_at",
  ];
  for (const key of Object.keys(obj)) {
    if (!allowed.includes(key)) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown release manifest field: ${key}`,
      );
    }
  }
  for (const field of [
    "release_id",
    "version",
    "release_channel",
    "deployment_profile",
    "manifest_digest",
    "created_at",
  ]) {
    if (typeof obj[field] !== "string" || (obj[field] as string).length === 0) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `${field} must be a non-empty string`,
      );
    }
  }
  const channel = assertKnownChannel(obj["release_channel"]);
  const profile = assertKnownProfile(obj["deployment_profile"]);
  if (!Array.isArray(obj["components"])) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "release manifest components must be an array",
    );
  }
  const components = obj["components"].map((item) =>
    parseSignedComponentWire(item),
  );
  const payload: Omit<ReleaseManifestWire, "manifest_digest"> = {
    schema_version: 1,
    release_id: obj["release_id"] as string,
    version: obj["version"] as string,
    release_channel: channel,
    deployment_profile: profile,
    components,
    created_at: obj["created_at"] as string,
  };
  const expectedDigest = manifestDigest(payload);
  if (obj["manifest_digest"] !== expectedDigest) {
    throw new ShipError(
      "VERIFICATION_FAILED",
      "release manifest digest mismatch: declared != computed",
    );
  }
  return { ...payload, manifest_digest: expectedDigest };
}

function parseSignedComponentWire(value: unknown): SignedComponentWire {
  if (typeof value !== "object" || value === null) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "signed component must be an object",
    );
  }
  const obj = value as Record<string, unknown>;
  const allowed = [
    "component_id",
    "name",
    "version",
    "artifact_ref",
    "digest",
    "signature",
    "sbom_ref",
    "license_ref",
    "size_bytes",
  ];
  for (const key of Object.keys(obj)) {
    if (!allowed.includes(key)) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown signed component field: ${key}`,
      );
    }
  }
  for (const field of [
    "component_id",
    "name",
    "version",
    "digest",
    "license_ref",
  ]) {
    if (typeof obj[field] !== "string" || (obj[field] as string).length === 0) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `${field} must be a non-empty string`,
      );
    }
  }
  if (
    typeof obj["size_bytes"] !== "number" ||
    (obj["size_bytes"] as number) <= 0
  ) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "size_bytes must be a positive number",
    );
  }
  if (typeof obj["artifact_ref"] !== "object" || obj["artifact_ref"] === null) {
    throw new ShipError("VALIDATION_FAILED", "artifact_ref must be an object");
  }
  if (typeof obj["sbom_ref"] !== "object" || obj["sbom_ref"] === null) {
    throw new ShipError("VALIDATION_FAILED", "sbom_ref must be an object");
  }
  if (typeof obj["signature"] !== "object" || obj["signature"] === null) {
    throw new ShipError("VALIDATION_FAILED", "signature must be an object");
  }
  const signature = obj["signature"] as Record<string, unknown>;
  for (const field of ["algorithm", "key_id", "value_b64"]) {
    if (
      typeof signature[field] !== "string" ||
      (signature[field] as string).length === 0
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `signature.${field} must be a non-empty string`,
      );
    }
  }
  return {
    component_id: obj["component_id"] as string,
    name: obj["name"] as string,
    version: obj["version"] as string,
    artifact_ref: obj["artifact_ref"] as unknown as ObjectRef,
    digest: obj["digest"] as string,
    signature: {
      algorithm: signature["algorithm"] as string,
      key_id: signature["key_id"] as string,
      value_b64: signature["value_b64"] as string,
    },
    sbom_ref: obj["sbom_ref"] as unknown as ObjectRef,
    license_ref: obj["license_ref"] as string,
    size_bytes: obj["size_bytes"] as number,
  };
}
