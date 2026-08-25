/**
 * EP-042 M5 OFFLINE installation (SPEC-016 behavior 5; ExecPlan M5
 * fence I/J).
 *
 * OFFLINE BUNDLE VERIFIED != OFFLINE INSTALL VERIFIED. installBundleOffline
 * composes the REAL M4 transactional installer (installers/ ->
 * installRelease) with artifact bytes read from the LOCAL bundle only:
 *
 *   bundle (local files)
 *     -> verifyBundle (digest-bound, fail-closed)
 *     -> read artifact bytes from bundle files (no transport)
 *     -> digest-bound component mapping (component.digest == item.digest)
 *     -> M4 installRelease (manifest validation, backup-before-update,
 *        staged replacement, atomic switch, verification)
 *
 * There is NO release transport in this path: no S3 client, no fetch,
 * no network. The bundle IS the artifact source. Installing from a
 * bundle therefore works with the release transport absent (fence J:
 * offline must actually mean offline).
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseReleaseManifest } from "@nexus/setup";
import { installRelease, type InstallComponent } from "@nexus/installers";
import { BundleError } from "./errors";
import {
  BUNDLE_MANIFEST_NAME,
  BUNDLE_RELEASE_MANIFEST_NAME,
  type BundleManifestWire,
  type BundleVerificationResult,
} from "./model";
import { bundleFilePath, verifyBundle } from "./verify";

export interface OfflineInstallOptions {
  bundleDir: string;
  installRoot: string;
  stagingRoot: string;
  backupRoot: string;
  quarantineRoot: string;
  journalRoot: string;
  releaseId: string;
  installId: string;
  runId: string;
  gitCommit: string;
  /** component_id -> relative install path (e.g. comp-1 -> bin/nexus-core). */
  componentPaths: Record<string, string>;
  /** Optional pre-verified bundle result (skips re-verification). */
  preVerified?: BundleVerificationResult;
}

export interface OfflineInstallResult {
  install: Awaited<ReturnType<typeof installRelease>>;
  verification: BundleVerificationResult;
  offline: {
    transport_required: false;
    source: "local-bundle-only";
    componentsResolved: string[];
  };
}

function readBundleManifest(bundleDir: string): BundleManifestWire {
  return JSON.parse(
    readFileSync(join(bundleDir, BUNDLE_MANIFEST_NAME), "utf8"),
  ) as BundleManifestWire;
}

/**
 * Install a release from a verified local bundle. The bundle must
 * verify first (or the caller supplies a pre-verified result from the
 * same run); an unverified bundle is denied before any mutation.
 */
export async function installBundleOffline(
  opts: OfflineInstallOptions,
): Promise<OfflineInstallResult> {
  const verification =
    opts.preVerified ?? (await verifyBundle({ bundleDir: opts.bundleDir }));

  const manifestWire = readFileSync(
    join(opts.bundleDir, BUNDLE_RELEASE_MANIFEST_NAME),
    "utf8",
  );
  const manifest = parseReleaseManifest(JSON.parse(manifestWire));

  // Digest-bound component mapping: for every declared component, find
  // the bundle item (IMAGE/MODEL payload) whose digest EXACTLY matches
  // the component's declared digest. No name guessing; a component with
  // no matching bundle payload is an unavailable dependency.
  const bundleItems = readBundleManifest(opts.bundleDir).contents;
  const byDigest = new Map<string, { kind: string; name: string }>();
  for (const item of bundleItems) {
    if (item.kind === "IMAGE" || item.kind === "MODEL") {
      byDigest.set(item.digest, item);
    }
  }

  const components: InstallComponent[] = [];
  const resolved: string[] = [];
  for (const component of manifest.components) {
    const item = byDigest.get(component.digest);
    if (item === undefined) {
      throw new BundleError(
        "BUNDLE_MISSING_FILE",
        `bundle does not contain artifact for declared component ${component.component_id}`,
        { componentId: component.component_id },
      );
    }
    const filePath = bundleFilePath(opts.bundleDir, item.kind, item.name);
    const bytes = readFileSync(filePath);
    const targetPath = opts.componentPaths[component.component_id];
    if (targetPath === undefined) {
      throw new BundleError(
        "BUNDLE_INVALID",
        `no install path supplied for component ${component.component_id}`,
        { componentId: component.component_id },
      );
    }
    components.push({
      componentId: component.component_id,
      declaredDigest: component.digest,
      bytes: new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength),
      path: targetPath,
    });
    resolved.push(component.component_id);
  }

  const install = await installRelease({
    installRoot: opts.installRoot,
    stagingRoot: opts.stagingRoot,
    backupRoot: opts.backupRoot,
    quarantineRoot: opts.quarantineRoot,
    journalRoot: opts.journalRoot,
    releaseId: opts.releaseId,
    installId: opts.installId,
    runId: opts.runId,
    gitCommit: opts.gitCommit,
    manifestWire: JSON.parse(manifestWire) as Record<string, unknown>,
    components,
  });

  return {
    install,
    verification,
    offline: {
      transport_required: false,
      source: "local-bundle-only",
      componentsResolved: resolved,
    },
  };
}
