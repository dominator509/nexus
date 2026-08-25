/**
 * EP-042 M5 offline bundle PRODUCTION (SPEC-016 behavior 5).
 *
 * Produces a REAL bundle from REAL files:
 *   bundle/
 *     bundle-manifest.json      canonical OfflineBundle wire (digest-bound)
 *     release-manifest.json     canonical release manifest (validated)
 *     images/... models/... licenses/... sboms/... migrations/... recovery/...
 *
 * Every payload is copied with real bytes; every declared digest is the
 * real sha256 of the copied bytes. The release manifest is validated
 * through the canonical M1/M2 surface before it may enter a bundle
 * (an unverifiable release is never bundled).
 *
 * OFFLINE BUNDLE EXISTS != OFFLINE BUNDLE VERIFIED: production only
 * creates the artifact; verification is a separate step (verify.ts).
 */

import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join, relative } from "node:path";
import {
  contentDigest,
  parseReleaseManifest,
  sha256Hex,
  verifyManifestDigestBinding,
} from "@nexus/setup";
import { BundleError } from "./errors";
import {
  BUNDLE_KIND_DIRS,
  BUNDLE_MANIFEST_NAME,
  BUNDLE_RELEASE_MANIFEST_NAME,
  BUNDLE_SCHEMA_VERSION,
  type BundleItemWire,
  type BundleManifestWire,
  type BundleProduceInput,
  type BundleProduceResult,
} from "./model";

function kindDir(kind: BundleItemWire["kind"]): string {
  return BUNDLE_KIND_DIRS[kind];
}

function requireRealFile(path: string, label: string): void {
  const bytes = readFileSync(path);
  if (bytes.length === 0) {
    throw new BundleError("BUNDLE_INVALID", `payload for ${label} is empty`, {
      label,
    });
  }
}

/**
 * Produce a real offline bundle. The release manifest must already be
 * digest-bound (manifest_digest matches its own content); a manifest
 * whose binding is MISSING or MISMATCH is refused (fail-closed).
 */
export async function produceBundle(
  input: BundleProduceInput,
): Promise<BundleProduceResult> {
  const { bundleDir, bundleId, releaseId, releaseManifestWire } = input;
  if (bundleId.length === 0) {
    throw new BundleError("BUNDLE_INVALID", "bundle_id must be non-empty");
  }
  if (releaseId.length === 0) {
    throw new BundleError("BUNDLE_INVALID", "release_id must be non-empty");
  }

  // Validate the release manifest through the canonical M1/M2 surface.
  let manifest;
  try {
    manifest = parseReleaseManifest(JSON.parse(releaseManifestWire));
  } catch (error) {
    throw new BundleError(
      "MANIFEST_INVALID",
      `release manifest failed validation: ${(error as Error).message}`,
      { releaseId },
    );
  }
  const binding = await verifyManifestDigestBinding(manifest);
  if (binding !== "VERIFIED") {
    throw new BundleError(
      "MANIFEST_INVALID",
      `release manifest digest binding is ${binding}; unverified releases are never bundled`,
      { releaseId },
    );
  }
  const manifestDigest = manifest.manifest_digest!;

  // The bundle's release id must match the manifest's release id.
  if (manifest.release_id !== releaseId) {
    throw new BundleError(
      "WRONG_RELEASE_ID",
      `bundle release_id ${releaseId} does not match manifest release_id ${manifest.release_id}`,
    );
  }

  // Every declared component must have artifact payload bytes supplied.
  const artifactKinds = new Set<string>();
  for (const component of manifest.components) {
    const artifact = input.artifacts[component.component_id];
    if (artifact === undefined) {
      throw new BundleError(
        "BUNDLE_INVALID",
        `no artifact payload supplied for declared component ${component.component_id}`,
        { componentId: component.component_id },
      );
    }
    requireRealFile(
      artifact.payloadPath,
      `component ${component.component_id}`,
    );
    artifactKinds.add(artifact.kind);
  }
  if (!artifactKinds.has("IMAGE") || !artifactKinds.has("MODEL")) {
    throw new BundleError(
      "BUNDLE_INVALID",
      "release artifacts must include at least one IMAGE and one MODEL payload",
    );
  }

  // Create the bundle directory layout.
  for (const dir of Object.values(BUNDLE_KIND_DIRS)) {
    mkdirSync(join(bundleDir, dir), { recursive: true });
  }

  const items: BundleItemWire[] = [];
  const filesWritten: string[] = [];
  const writePayload = async (
    kind: BundleItemWire["kind"],
    name: string,
    sourcePath: string,
  ): Promise<BundleItemWire> => {
    const target = join(bundleDir, kindDir(kind), basename(name));
    copyFileSync(sourcePath, target);
    filesWritten.push(relative(bundleDir, target));
    const targetBytes = readFileSync(target);
    const digest = await sha256Hex(
      new Uint8Array(
        targetBytes.buffer as ArrayBuffer,
        targetBytes.byteOffset,
        targetBytes.byteLength,
      ) as Uint8Array<ArrayBuffer>,
    );
    return { kind, name: basename(name), digest: `sha256:${digest}` };
  };

  // Component artifacts (IMAGE/MODEL payloads).
  for (const component of manifest.components) {
    const artifact = input.artifacts[component.component_id]!;
    const item = await writePayload(
      artifact.kind,
      artifact.name,
      artifact.payloadPath,
    );
    items.push(item);
  }

  // SBOM / license / migration / recovery payloads.
  for (const sbom of input.sbomPayloads) {
    requireRealFile(sbom.payloadPath, `sbom ${sbom.name}`);
    items.push(await writePayload("SBOM", sbom.name, sbom.payloadPath));
  }
  for (const license of input.licensePayloads) {
    requireRealFile(license.payloadPath, `license ${license.name}`);
    items.push(
      await writePayload("LICENSE", license.name, license.payloadPath),
    );
  }
  for (const migration of input.migrationPayloads) {
    requireRealFile(migration.payloadPath, `migration ${migration.name}`);
    items.push(
      await writePayload("MIGRATION", migration.name, migration.payloadPath),
    );
  }
  for (const tool of input.recoveryToolPayloads) {
    requireRealFile(tool.payloadPath, `recovery tool ${tool.name}`);
    items.push(
      await writePayload("RECOVERY_TOOL", tool.name, tool.payloadPath),
    );
  }

  // The release manifest itself is part of the bundle.
  writeFileSync(
    join(bundleDir, BUNDLE_RELEASE_MANIFEST_NAME),
    releaseManifestWire,
  );
  filesWritten.push(BUNDLE_RELEASE_MANIFEST_NAME);

  // Required content kinds (M1 parity: IMAGE/MODEL/LICENSE/SBOM).
  const kinds = new Set(items.map((item) => item.kind));
  for (const required of ["IMAGE", "MODEL", "LICENSE", "SBOM"] as const) {
    if (!kinds.has(required)) {
      throw new BundleError(
        "BUNDLE_REQUIRED_KIND_MISSING",
        `offline bundle missing required content kind ${required}`,
      );
    }
  }
  if (input.sbomPayloads.length === 0) {
    throw new BundleError(
      "BUNDLE_REQUIRED_KIND_MISSING",
      "offline bundle must reference at least one SBOM",
    );
  }

  const sbomNames = input.sbomPayloads.map((sbom) => basename(sbom.name));
  const licenseNames = input.licensePayloads.map((l) => basename(l.name));
  const migrationNames = input.migrationPayloads.map((m) => basename(m.name));

  const manifestRef = {
    backend: "local",
    key: BUNDLE_RELEASE_MANIFEST_NAME,
  };

  // Build the canonical bundle manifest with the self-binding digest:
  // bundle_digest is EXCLUDED from its own content digest
  // (strip-then-digest, M1 parity with manifest_digest).
  const base = {
    schema_version: BUNDLE_SCHEMA_VERSION,
    bundle_id: bundleId,
    release_id: releaseId,
    contents: items,
    manifest_ref: manifestRef,
    sbom_refs: sbomNames,
    license_refs: licenseNames,
    migrations: migrationNames,
  };
  const bundleDigest = (
    await contentDigest(base as unknown as Record<string, unknown>)
  ).asString();
  const finalManifest: BundleManifestWire = {
    ...base,
    bundle_digest: bundleDigest,
  };
  writeFileSync(
    join(bundleDir, BUNDLE_MANIFEST_NAME),
    JSON.stringify(finalManifest, null, 2),
  );
  filesWritten.push(BUNDLE_MANIFEST_NAME);

  return {
    bundleDir,
    bundleId,
    releaseId,
    manifestDigest,
    bundleDigest,
    items,
    filesWritten,
  };
}
