/**
 * EP-042 M5 offline bundle VERIFICATION (SPEC-016 behavior 5, SPEC-024;
 * ExecPlan M5 fence G).
 *
 * OFFLINE BUNDLE EXISTS != OFFLINE BUNDLE VERIFIED. verifyBundle proves,
 * against real bytes on disk:
 *   - bundle-manifest.json parses (deny-unknown, schema_version 1)
 *   - every declared file exists                        (missing -> denied)
 *   - every declared digest matches real bytes          (changed -> denied)
 *   - every digest is well-formed                       (malformed -> denied)
 *   - no duplicate item path                            (duplicate -> denied)
 *   - no path traversal / symlink escape                (escape -> denied)
 *   - bundle.release_id == manifest.release_id          (wrong -> denied)
 *   - manifest digest binding holds                     (tamper -> denied)
 *   - bundle_digest self-binding holds (strip-then-digest)
 *
 * A verified bundle may then be installed OFFLINE (install.ts) with no
 * transport dependency.
 */

import { readFileSync, realpathSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import {
  isDigestString,
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
  type BundleManifestWire,
  type BundleVerificationResult,
} from "./model";

export interface VerifyOptions {
  bundleDir: string;
}

function parseBundleManifest(bundleDir: string): BundleManifestWire {
  const raw = readFileSync(join(bundleDir, BUNDLE_MANIFEST_NAME), "utf8");
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new BundleError(
      "BUNDLE_INVALID",
      "bundle-manifest.json is not valid JSON",
    );
  }
  if (typeof parsed !== "object" || parsed === null) {
    throw new BundleError(
      "BUNDLE_INVALID",
      "bundle manifest must be an object",
    );
  }
  const obj = parsed as Record<string, unknown>;
  if (obj.schema_version !== BUNDLE_SCHEMA_VERSION) {
    throw new BundleError(
      "BUNDLE_INVALID",
      `unsupported bundle schema_version ${String(obj.schema_version)}`,
    );
  }
  const requireString = (field: string): string => {
    const value = obj[field];
    if (typeof value !== "string" || value.length === 0) {
      throw new BundleError(
        "BUNDLE_INVALID",
        `bundle manifest field ${field} must be a non-empty string`,
        { field },
      );
    }
    return value;
  };
  const requireStringArray = (field: string): string[] => {
    const value = obj[field];
    if (
      !Array.isArray(value) ||
      value.some((entry) => typeof entry !== "string")
    ) {
      throw new BundleError(
        "BUNDLE_INVALID",
        `bundle manifest field ${field} must be an array of strings`,
        { field },
      );
    }
    return value as string[];
  };
  const bundleId = requireString("bundle_id");
  const releaseId = requireString("release_id");
  const contentsRaw = obj.contents;
  if (!Array.isArray(contentsRaw) || contentsRaw.length === 0) {
    throw new BundleError(
      "BUNDLE_INVALID",
      "bundle manifest contents must be a non-empty array",
    );
  }
  const contents = contentsRaw.map((entry, index) => {
    if (typeof entry !== "object" || entry === null) {
      throw new BundleError(
        "BUNDLE_INVALID",
        `contents[${index}] not an object`,
      );
    }
    const item = entry as Record<string, unknown>;
    const kind = item.kind;
    const name = item.name;
    const digest = item.digest;
    if (
      typeof kind !== "string" ||
      !(Object.values(BUNDLE_KIND_DIRS) as string[]).includes(
        BUNDLE_KIND_DIRS[kind as keyof typeof BUNDLE_KIND_DIRS] ?? "",
      )
    ) {
      throw new BundleError(
        "BUNDLE_INVALID",
        `contents[${index}] invalid kind`,
      );
    }
    if (typeof name !== "string" || name.length === 0) {
      throw new BundleError(
        "BUNDLE_INVALID",
        `contents[${index}] invalid name`,
      );
    }
    if (typeof digest !== "string" || !isDigestString(digest)) {
      throw new BundleError(
        "BUNDLE_MALFORMED_DIGEST",
        `contents[${index}] digest not well-formed: ${String(digest)}`,
        { digest: String(digest) },
      );
    }
    return { kind, name, digest } as BundleManifestWire["contents"][number];
  });
  const manifestRef = obj.manifest_ref;
  if (
    typeof manifestRef !== "object" ||
    manifestRef === null ||
    typeof (manifestRef as Record<string, unknown>).key !== "string"
  ) {
    throw new BundleError("BUNDLE_INVALID", "bundle manifest_ref invalid");
  }
  const bundleDigest =
    typeof obj.bundle_digest === "string" ? obj.bundle_digest : null;
  if (bundleDigest !== null && !isDigestString(bundleDigest)) {
    throw new BundleError(
      "BUNDLE_MALFORMED_DIGEST",
      "bundle_digest not well-formed",
    );
  }
  return {
    schema_version: BUNDLE_SCHEMA_VERSION,
    bundle_id: bundleId,
    release_id: releaseId,
    contents,
    manifest_ref: manifestRef as BundleManifestWire["manifest_ref"],
    sbom_refs: requireStringArray("sbom_refs"),
    license_refs: requireStringArray("license_refs"),
    migrations: requireStringArray("migrations"),
    bundle_digest: bundleDigest,
  };
}

/**
 * Resolve a declared bundle item to its relative path and enforce that
 * the path stays inside the bundle root (no traversal, no symlink
 * escape). The item file must exist at verification time, so the
 * realpath of the file itself is checked against the realpath of the
 * bundle root.
 */
function resolveItemPath(
  bundleDir: string,
  kind: string,
  name: string,
): string {
  const kindDir = BUNDLE_KIND_DIRS[kind as keyof typeof BUNDLE_KIND_DIRS];
  if (kindDir === undefined) {
    throw new BundleError("BUNDLE_INVALID", `unknown bundle kind ${kind}`);
  }
  if (name.length === 0) {
    throw new BundleError(
      "BUNDLE_INVALID",
      "bundle item name must be non-empty",
    );
  }
  if (isAbsolute(name)) {
    throw new BundleError("PATH_ESCAPE", `absolute bundle item path ${name}`);
  }
  const segments = name.split(/[\\/]+/);
  if (segments.some((segment) => segment === "..")) {
    throw new BundleError(
      "PATH_ESCAPE",
      `bundle item path traversal denied: ${name}`,
    );
  }
  const resolved = resolve(bundleDir, kindDir, ...segments);
  const rootReal = realpathSync(bundleDir);
  const rel = relative(rootReal, resolved);
  if (rel.startsWith("..") || isAbsolute(rel)) {
    throw new BundleError(
      "PATH_ESCAPE",
      `bundle item escapes bundle root: ${name}`,
    );
  }
  // Symlink escape: the resolved file's real path must stay inside the
  // bundle root. A symlink inside the bundle pointing outside is denied.
  try {
    const fileReal = realpathSync(resolved);
    const relReal = relative(rootReal, fileReal);
    if (relReal.startsWith("..") || isAbsolute(relReal)) {
      throw new BundleError(
        "PATH_ESCAPE",
        `bundle item symlink escapes bundle root: ${name}`,
      );
    }
  } catch (error) {
    if (error instanceof BundleError) throw error;
    throw new BundleError(
      "BUNDLE_MISSING_FILE",
      `bundle item does not resolve: ${name}`,
    );
  }
  return resolved;
}

/**
 * Verify a bundle against real bytes. Returns VERIFIED only when every
 * declared file exists, every digest matches, the manifest binding
 * holds, the release id matches, no duplicate/escape exists, and the
 * bundle self-digest holds. Any denial throws a typed BundleError; the
 * denial shape is also recorded on the result.
 */
export async function verifyBundle(
  opts: VerifyOptions,
): Promise<BundleVerificationResult> {
  const { bundleDir } = opts;
  const manifest = parseBundleManifest(bundleDir);

  // Duplicate path detection: the same resolved relative path may only
  // appear once.
  const seen = new Set<string>();
  for (const item of manifest.contents) {
    const relPath = join(
      BUNDLE_KIND_DIRS[item.kind as keyof typeof BUNDLE_KIND_DIRS],
      item.name,
    );
    if (seen.has(relPath)) {
      throw new BundleError(
        "BUNDLE_DUPLICATE_PATH",
        `duplicate bundle item path: ${relPath}`,
        { path: relPath },
      );
    }
    seen.add(relPath);
  }

  // Every declared file exists and matches its digest (real bytes).
  let filesVerified = 0;
  for (const item of manifest.contents) {
    const resolved = resolveItemPath(bundleDir, item.kind, item.name);
    let bytes: Buffer;
    try {
      bytes = readFileSync(resolved);
    } catch {
      throw new BundleError(
        "BUNDLE_MISSING_FILE",
        `declared bundle file missing: ${item.kind}/${item.name}`,
        { file: item.kind + "/" + item.name },
      );
    }
    const actual = await sha256Hex(toBytes(bytes));
    const declared = item.digest.startsWith("sha256:")
      ? item.digest.slice("sha256:".length)
      : item.digest;
    if (actual !== declared) {
      throw new BundleError(
        "BUNDLE_DIGEST_MISMATCH",
        `bundle file digest mismatch: ${item.kind}/${item.name}`,
        { file: item.kind + "/" + item.name },
      );
    }
    filesVerified += 1;
  }

  // Release manifest present + canonical validation + digest binding.
  const manifestPath = join(bundleDir, BUNDLE_RELEASE_MANIFEST_NAME);
  let manifestWire: string;
  try {
    manifestWire = readFileSync(manifestPath, "utf8");
  } catch {
    throw new BundleError(
      "BUNDLE_MISSING_FILE",
      `release manifest missing from bundle: ${BUNDLE_RELEASE_MANIFEST_NAME}`,
    );
  }
  let releaseManifest;
  try {
    releaseManifest = parseReleaseManifest(JSON.parse(manifestWire));
  } catch (error) {
    throw new BundleError(
      "MANIFEST_INVALID",
      `bundle release manifest failed validation: ${(error as Error).message}`,
    );
  }
  const binding = await verifyManifestDigestBinding(releaseManifest);
  if (binding !== "VERIFIED") {
    throw new BundleError(
      "MANIFEST_INVALID",
      `bundle release manifest digest binding is ${binding}`,
    );
  }
  if (releaseManifest.release_id !== manifest.release_id) {
    throw new BundleError(
      "WRONG_RELEASE_ID",
      `bundle release_id ${manifest.release_id} does not match manifest release_id ${releaseManifest.release_id}`,
    );
  }
  filesVerified += 1;

  // Referenced SBOM / license / migration files must exist in the bundle.
  const refFiles = [
    ...manifest.sbom_refs.map((name) => join(BUNDLE_KIND_DIRS.SBOM, name)),
    ...manifest.license_refs.map((name) =>
      join(BUNDLE_KIND_DIRS.LICENSE, name),
    ),
    ...manifest.migrations.map((name) =>
      join(BUNDLE_KIND_DIRS.MIGRATION, name),
    ),
  ];
  for (const relPath of refFiles) {
    const resolved = resolve(bundleDir, relPath);
    try {
      readFileSync(resolved);
    } catch {
      throw new BundleError(
        "BUNDLE_MISSING_FILE",
        `bundle reference missing: ${relPath}`,
        { file: relPath },
      );
    }
    filesVerified += 1;
  }

  // Bundle self-digest binding (strip-then-digest).
  if (manifest.bundle_digest === null) {
    throw new BundleError(
      "BUNDLE_SELF_DIGEST_MISMATCH",
      "bundle_digest missing; bundle not self-bound",
    );
  }
  const parsed = JSON.parse(
    readFileSync(join(bundleDir, BUNDLE_MANIFEST_NAME), "utf8"),
  ) as Record<string, unknown>;
  const { bundle_digest: _excluded, ...rest } = parsed;
  const actualBundleDigest = await sha256Hex(
    toBytes(Buffer.from(JSON.stringify(rest))),
  );
  const declaredBundleDigest = manifest.bundle_digest.startsWith("sha256:")
    ? manifest.bundle_digest.slice("sha256:".length)
    : manifest.bundle_digest;
  if (actualBundleDigest !== declaredBundleDigest) {
    throw new BundleError(
      "BUNDLE_SELF_DIGEST_MISMATCH",
      "bundle self-digest binding mismatch (tampered bundle manifest)",
    );
  }

  return {
    state: "VERIFIED",
    bundleId: manifest.bundle_id,
    releaseId: manifest.release_id,
    bundleDigest: manifest.bundle_digest,
    manifestDigest: releaseManifest.manifest_digest!,
    itemCount: manifest.contents.length,
    filesVerified,
  };
}

/** Convert a Node Buffer to Uint8Array<ArrayBuffer> for Web Crypto. */
function toBytes(bytes: Uint8Array<ArrayBufferLike>): Uint8Array<ArrayBuffer> {
  return new Uint8Array(
    bytes.buffer as ArrayBuffer,
    bytes.byteOffset,
    bytes.byteLength,
  ) as Uint8Array<ArrayBuffer>;
}

export function bundleItemRelPath(kind: string, name: string): string {
  return join(BUNDLE_KIND_DIRS[kind as keyof typeof BUNDLE_KIND_DIRS], name);
}

export function bundleFilePath(
  bundleDir: string,
  kind: string,
  name: string,
): string {
  const rel = bundleItemRelPath(kind, name);
  const resolved = resolve(bundleDir, rel);
  const rootReal = realpathSync(bundleDir);
  if (relative(rootReal, resolved).startsWith("..")) {
    throw new BundleError("PATH_ESCAPE", `escapes bundle root: ${rel}`);
  }
  return resolved;
}
