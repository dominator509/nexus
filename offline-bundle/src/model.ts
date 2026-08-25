/**
 * EP-042 M5 offline bundle model (SPEC-016 behavior 5, SPEC-024;
 * ADR-028 vocabulary: BundleKind IMAGE/MODEL/LICENSE/SBOM/MIGRATION/
 * RECOVERY_TOOL; canonical OfflineBundle shape from crates/nexus-release).
 *
 * The bundle-manifest.json written into a bundle is the canonical
 * OfflineBundle wire shape (snake_case, deny-unknown, schema_version 1),
 * adapted at the boundary from the M1 Rust contract exactly like the M2
 * update core adapts the other release interfaces.
 */

import type { BundleErrorShape } from "./errors";

export const BUNDLE_SCHEMA_VERSION = 1;

export const BUNDLE_KINDS = [
  "IMAGE",
  "MODEL",
  "LICENSE",
  "SBOM",
  "MIGRATION",
  "RECOVERY_TOOL",
] as const;

export type BundleKind = (typeof BUNDLE_KINDS)[number];

/** BundleKinds that a valid offline bundle MUST contain (M1 parity). */
export const BUNDLE_REQUIRED_KINDS: readonly BundleKind[] = [
  "IMAGE",
  "MODEL",
  "LICENSE",
  "SBOM",
];

/** Relative subdirectory inside the bundle for each content kind. */
export const BUNDLE_KIND_DIRS: Record<BundleKind, string> = {
  IMAGE: "images",
  MODEL: "models",
  LICENSE: "licenses",
  SBOM: "sboms",
  MIGRATION: "migrations",
  RECOVERY_TOOL: "recovery",
};

/** The canonical release manifest filename inside the bundle. */
export const BUNDLE_RELEASE_MANIFEST_NAME = "release-manifest.json";
/** The canonical bundle manifest filename inside the bundle. */
export const BUNDLE_MANIFEST_NAME = "bundle-manifest.json";

export interface ObjectRefWire {
  backend: string;
  key: string;
}

export interface BundleItemWire {
  kind: BundleKind;
  name: string;
  digest: string;
}

export interface BundleManifestWire {
  schema_version: number;
  bundle_id: string;
  release_id: string;
  contents: BundleItemWire[];
  manifest_ref: ObjectRefWire;
  sbom_refs: string[];
  license_refs: string[];
  migrations: string[];
  bundle_digest: string | null;
}

export interface BundleVerificationResult {
  state: "VERIFIED" | "MISMATCH" | "MISSING" | "MALFORMED";
  bundleId: string;
  releaseId: string;
  bundleDigest: string | null;
  manifestDigest: string;
  itemCount: number;
  filesVerified: number;
  denied?: BundleErrorShape;
}

export interface BundleProduceInput {
  bundleDir: string;
  bundleId: string;
  releaseId: string;
  /** Canonical release manifest wire JSON (validated + digest-bound). */
  releaseManifestWire: string;
  /** component_id -> artifact payload (kind IMAGE or MODEL). */
  artifacts: Record<
    string,
    { kind: "IMAGE" | "MODEL"; payloadPath: string; name: string }
  >;
  sbomPayloads: { name: string; payloadPath: string }[];
  licensePayloads: { name: string; payloadPath: string }[];
  migrationPayloads: { name: string; payloadPath: string }[];
  recoveryToolPayloads: { name: string; payloadPath: string }[];
}

export interface BundleProduceResult {
  bundleDir: string;
  bundleId: string;
  releaseId: string;
  manifestDigest: string;
  bundleDigest: string;
  items: BundleItemWire[];
  filesWritten: string[];
}
