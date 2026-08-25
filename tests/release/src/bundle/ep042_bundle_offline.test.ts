/**
 * EP-042 M5 offline bundle proofs (SPEC-016 behavior 5, SPEC-024;
 * ExecPlan M5 fence F/G/H/I/J/M/O/P).
 *
 * REAL bundle behavior, never mocks:
 * - bundle production from REAL files with REAL sha256 digests
 * - digest-bound verification: missing/changed/malformed digest,
 *   duplicate path, path traversal, symlink escape, wrong release id,
 *   tampered manifest, tampered bundle self-digest -> all denied
 * - OFFLINE install from a verified bundle with NO transport (the
 *   install path reads artifact bytes from local bundle files only;
 *   no S3 client, no fetch, no network is ever touched)
 * - rollback drill: prior state restored + verified BEFORE receipt
 * - evidence: current-run bound, redacted; stale/tampered rejected
 *
 * Every proof runs against REAL temporary roots under the test tmpdir
 * (never the host nexus tree) using the real filesystem.
 */

import { describe, expect, it } from "vitest";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { contentDigest, type Digest } from "@nexus/setup";
import {
  buildBundleEvidence,
  installBundleOffline,
  produceBundle,
  runRollbackDrill,
  validateEvidence,
  verifyBundle,
  type BundleError,
  BundleError as BundleErrorClass,
} from "@nexus/offline-bundle";

interface Fixture {
  base: string;
  manifestWire: string;
  manifestPath: string;
  artifactPaths: Record<string, string>;
  sbomPath: string;
  licensePath: string;
  migrationPath: string;
  recoveryPath: string;
  releaseId: string;
  componentDigests: Record<string, string>;
  componentNames: Record<string, string>;
}

async function sha256Of(bytes: Uint8Array<ArrayBuffer>): Promise<string> {
  const digestBuffer = await globalThis.crypto.subtle.digest("SHA-256", bytes);
  const out = new Uint8Array(digestBuffer);
  let s = "";
  for (const b of out) s += b.toString(16).padStart(2, "0");
  return s;
}

function freshBase(label: string): string {
  return join(
    tmpdir(),
    `nexus-ep042-m5-${label}-${Date.now()}-${Math.floor(Math.random() * 1e6)}`,
  );
}

async function makeFixture(label: string): Promise<Fixture> {
  const base = freshBase(label);
  const artifactsDir = join(base, "artifacts");
  mkdirSync(artifactsDir, { recursive: true });
  const sbomDir = join(base, "sboms");
  mkdirSync(sbomDir, { recursive: true });

  const c1 = new TextEncoder().encode("nexus-core-v1 real offline bytes");
  const c2 = new TextEncoder().encode("nexus-model-v2 real offline bytes");
  const c1Path = join(artifactsDir, "artifact-comp-1");
  const c2Path = join(artifactsDir, "artifact-comp-2");
  writeFileSync(c1Path, c1);
  writeFileSync(c2Path, c2);
  const d1 = `sha256:${await sha256Of(c1)}`;
  const d2 = `sha256:${await sha256Of(c2)}`;

  const sbomPath = join(sbomDir, "sbom.json");
  const licensePath = join(base, "LICENSE");
  const migrationPath = join(base, "migration-1.sql");
  const recoveryPath = join(base, "recover.sh");
  writeFileSync(sbomPath, '{"bomFormat":"CycloneDX","version":1}');
  writeFileSync(licensePath, "MIT License text (fixture)");
  writeFileSync(migrationPath, "ALTER TABLE nexus ADD COLUMN offline int;");
  writeFileSync(recoveryPath, "#!/bin/sh\necho recover\n");

  const wire = {
    schema_version: 1,
    release_id: "release-1",
    version: "1.0.0",
    channel: "STABLE",
    components: [
      {
        component_id: "comp-1",
        name: "component-comp-1",
        version: "1.0.0",
        artifact_ref: { backend: "local", key: "artifact-comp-1" },
        digest: d1,
        signature: {
          algorithm: "ED25519",
          key_id: "key-test-1",
          value_b64: "AAAA01BBBB01",
        },
        sbom_ref: { backend: "local", key: "sbom-comp-1" },
        license_ref: "MIT",
        size_bytes: c1.byteLength,
      },
      {
        component_id: "comp-2",
        name: "component-comp-2",
        version: "2.0.0",
        artifact_ref: { backend: "local", key: "artifact-comp-2" },
        digest: d2,
        signature: {
          algorithm: "ED25519",
          key_id: "key-test-1",
          value_b64: "AAAA01BBBB01",
        },
        sbom_ref: { backend: "local", key: "sbom-comp-2" },
        license_ref: "MIT",
        size_bytes: c2.byteLength,
      },
    ],
    compatibility: {
      matrix_id: "matrix-1",
      schema_version: 1,
      entries: [
        {
          component_id: "comp-1",
          version: "1.0.0",
          min_version: "1.0.0",
          max_version: "1.9.9",
          supported_profiles: [
            "MANAGED",
            "BYOC",
            "EXISTING_SSH",
            "HYBRID",
            "FULLY_LOCAL",
          ],
        },
        {
          component_id: "comp-2",
          version: "2.0.0",
          min_version: "2.0.0",
          max_version: "2.9.9",
          supported_profiles: [
            "MANAGED",
            "BYOC",
            "EXISTING_SSH",
            "HYBRID",
            "FULLY_LOCAL",
          ],
        },
      ],
    },
    sbom_ref: { backend: "local", key: "sbom-root" },
    license_refs: ["MIT"],
    created_at: "2026-08-25T00:00:00Z",
  };
  const digest = (await contentDigest(wire)) as Digest & { asString(): string };
  const manifestWire = JSON.stringify(
    { ...wire, manifest_digest: digest.asString() },
    null,
    2,
  );
  const manifestPath = join(base, "manifest.json");
  writeFileSync(manifestPath, manifestWire);

  return {
    base,
    manifestWire,
    manifestPath,
    artifactPaths: { "comp-1": c1Path, "comp-2": c2Path },
    sbomPath,
    licensePath,
    migrationPath,
    recoveryPath,
    releaseId: "release-1",
    componentDigests: { "comp-1": d1, "comp-2": d2 },
    componentNames: {
      "comp-1": "artifact-comp-1",
      "comp-2": "artifact-comp-2",
    },
  };
}

async function makeBundle(
  label: string,
): Promise<{ fixture: Fixture; bundleDir: string; bundleId: string }> {
  const fixture = await makeFixture(label);
  const bundleDir = join(fixture.base, "bundle");
  const bundleId = `bundle-${label}`;
  await produceBundle({
    bundleDir,
    bundleId,
    releaseId: fixture.releaseId,
    releaseManifestWire: fixture.manifestWire,
    artifacts: {
      "comp-1": {
        kind: "IMAGE",
        payloadPath: fixture.artifactPaths["comp-1"]!,
        name: "artifact-comp-1",
      },
      "comp-2": {
        kind: "MODEL",
        payloadPath: fixture.artifactPaths["comp-2"]!,
        name: "artifact-comp-2",
      },
    },
    sbomPayloads: [{ name: "sbom.json", payloadPath: fixture.sbomPath }],
    licensePayloads: [{ name: "LICENSE", payloadPath: fixture.licensePath }],
    migrationPayloads: [
      { name: "migration-1.sql", payloadPath: fixture.migrationPath },
    ],
    recoveryToolPayloads: [
      { name: "recover.sh", payloadPath: fixture.recoveryPath },
    ],
  });
  return { fixture, bundleDir, bundleId };
}

function expectBundleError(
  fn: () => Promise<unknown>,
  code: string,
): Promise<void> {
  return fn().then(
    () => {
      throw new Error(`expected BundleError ${code} but call succeeded`);
    },
    (error: unknown) => {
      expect(error).toBeInstanceOf(BundleErrorClass);
      expect((error as BundleError).code).toBe(code);
    },
  );
}

const RUN = {
  runId: `ep042-m5-${Date.now()}`,
  gitCommit: "test-commit",
  releaseId: "release-1",
  installId: "install-1",
};

describe("ep042_bundle offline bundle real proofs", () => {
  it("ep042_bundle_production_creates_real_bundle", async () => {
    const { fixture, bundleDir, bundleId } = await makeBundle("prod");
    try {
      const manifest = JSON.parse(
        readFileSync(join(bundleDir, "bundle-manifest.json"), "utf8"),
      ) as {
        bundle_id: string;
        release_id: string;
        contents: unknown[];
        bundle_digest: string;
      };
      expect(manifest.bundle_id).toBe(bundleId);
      expect(manifest.release_id).toBe(fixture.releaseId);
      expect(manifest.contents.length).toBeGreaterThanOrEqual(4);
      expect(manifest.bundle_digest).toMatch(/^sha256:[0-9a-f]{64}$/);
      // Real payloads exist with real bytes.
      const coreBytes = readFileSync(
        join(bundleDir, "images", "artifact-comp-1"),
      );
      expect(coreBytes.toString()).toBe("nexus-core-v1 real offline bytes");
      const modelBytes = readFileSync(
        join(bundleDir, "models", "artifact-comp-2"),
      );
      expect(modelBytes.toString()).toBe("nexus-model-v2 real offline bytes");
      expect(existsSync(join(bundleDir, "sboms", "sbom.json"))).toBe(true);
      expect(existsSync(join(bundleDir, "licenses", "LICENSE"))).toBe(true);
      expect(existsSync(join(bundleDir, "migrations", "migration-1.sql"))).toBe(
        true,
      );
      expect(existsSync(join(bundleDir, "recovery", "recover.sh"))).toBe(true);
      expect(existsSync(join(bundleDir, "release-manifest.json"))).toBe(true);
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_verification_passes", async () => {
    const { fixture, bundleDir } = await makeBundle("verify");
    try {
      const result = await verifyBundle({ bundleDir });
      expect(result.state).toBe("VERIFIED");
      expect(result.releaseId).toBe("release-1");
      expect(result.filesVerified).toBeGreaterThanOrEqual(6);
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_missing_file_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("missing");
    try {
      rmSync(join(bundleDir, "images", "artifact-comp-1"));
      await expectBundleError(
        () => verifyBundle({ bundleDir }),
        "BUNDLE_MISSING_FILE",
      );
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_changed_file_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("changed");
    try {
      writeFileSync(
        join(bundleDir, "models", "artifact-comp-2"),
        "tampered bytes change the digest",
      );
      await expectBundleError(
        () => verifyBundle({ bundleDir }),
        "BUNDLE_DIGEST_MISMATCH",
      );
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_malformed_digest_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("malformed");
    try {
      const manifestPath = join(bundleDir, "bundle-manifest.json");
      const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
        contents: { digest: string }[];
      };
      manifest.contents[0]!.digest = "sha256:abc";
      writeFileSync(manifestPath, JSON.stringify(manifest));
      await expectBundleError(
        () => verifyBundle({ bundleDir }),
        "BUNDLE_MALFORMED_DIGEST",
      );
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_duplicate_path_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("dupe");
    try {
      const manifestPath = join(bundleDir, "bundle-manifest.json");
      const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
        contents: unknown[];
      };
      manifest.contents.push(manifest.contents[0]); // same kind+name -> duplicate path
      writeFileSync(manifestPath, JSON.stringify(manifest));
      await expectBundleError(
        () => verifyBundle({ bundleDir }),
        "BUNDLE_DUPLICATE_PATH",
      );
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_path_traversal_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("traversal");
    try {
      const manifestPath = join(bundleDir, "bundle-manifest.json");
      const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
        contents: { name: string }[];
      };
      manifest.contents[0]!.name = "../../escape";
      writeFileSync(manifestPath, JSON.stringify(manifest));
      await expectBundleError(() => verifyBundle({ bundleDir }), "PATH_ESCAPE");
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_symlink_escape_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("symlink");
    try {
      const outside = join(fixture.base, "outside-secret.txt");
      writeFileSync(outside, "host file outside the bundle");
      rmSync(join(bundleDir, "images", "artifact-comp-1"));
      symlinkSync(outside, join(bundleDir, "images", "artifact-comp-1"));
      await expectBundleError(() => verifyBundle({ bundleDir }), "PATH_ESCAPE");
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_wrong_release_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("wrongrelease");
    try {
      // Internally consistent bundle manifest (self-digest recomputed)
      // but claiming a DIFFERENT release than the release manifest.
      const manifestPath = join(bundleDir, "bundle-manifest.json");
      const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
        release_id: string;
        bundle_digest: string | null;
      };
      manifest.release_id = "release-other";
      const { bundle_digest: _omit, ...rest } = manifest;
      const digest = await contentDigest(
        rest as unknown as Record<string, unknown>,
      );
      manifest.bundle_digest = (
        digest as Digest & { asString(): string }
      ).asString();
      writeFileSync(manifestPath, JSON.stringify(manifest));
      await expectBundleError(
        () => verifyBundle({ bundleDir }),
        "WRONG_RELEASE_ID",
      );
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_manifest_tamper_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("tampermanifest");
    try {
      const manifestPath = join(bundleDir, "release-manifest.json");
      const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
        version: string;
      };
      manifest.version = "9.9.9"; // breaks the digest binding
      writeFileSync(manifestPath, JSON.stringify(manifest));
      await expectBundleError(
        () => verifyBundle({ bundleDir }),
        "MANIFEST_INVALID",
      );
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_self_digest_tamper_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("tamperself");
    try {
      const manifestPath = join(bundleDir, "bundle-manifest.json");
      const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
        bundle_id: string;
      };
      manifest.bundle_id = "bundle-other"; // no digest recompute -> mismatch
      writeFileSync(manifestPath, JSON.stringify(manifest));
      await expectBundleError(
        () => verifyBundle({ bundleDir }),
        "BUNDLE_SELF_DIGEST_MISMATCH",
      );
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_offline_install_succeeds", async () => {
    const { fixture, bundleDir } = await makeBundle("install");
    try {
      const installRoot = join(fixture.base, "install");
      mkdirSync(installRoot, { recursive: true });
      writeFileSync(join(installRoot, "prior-state"), "prior-state-bytes");
      const result = await installBundleOffline({
        bundleDir,
        installRoot,
        stagingRoot: join(fixture.base, "staging"),
        backupRoot: join(fixture.base, "backup"),
        quarantineRoot: join(fixture.base, "quarantine"),
        journalRoot: join(fixture.base, "journal"),
        releaseId: RUN.releaseId,
        installId: RUN.installId,
        runId: RUN.runId,
        gitCommit: RUN.gitCommit,
        componentPaths: {
          "comp-1": "bin/nexus-core",
          "comp-2": "models/nexus-model",
        },
      });
      expect(result.offline.transport_required).toBe(false);
      expect(result.offline.source).toBe("local-bundle-only");
      expect(result.offline.componentsResolved.sort()).toEqual([
        "comp-1",
        "comp-2",
      ]);
      // cmp-verified installed bytes against the REAL bundle payloads.
      expect(
        readFileSync(join(installRoot, "bin", "nexus-core")).toString(),
      ).toBe("nexus-core-v1 real offline bytes");
      expect(
        readFileSync(join(installRoot, "models", "nexus-model")).toString(),
      ).toBe("nexus-model-v2 real offline bytes");
      expect(result.install.backup).toBeDefined();
      expect(result.install.backup!.state).toBe("VERIFIED");
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_offline_install_component_missing_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("missingcomp");
    try {
      // Drop comp-2's payload item from the bundle manifest (recompute
      // self-digest so the bundle is internally consistent), while the
      // release manifest still declares comp-2. Install must deny with
      // the component unavailable in the bundle.
      const manifestPath = join(bundleDir, "bundle-manifest.json");
      const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as {
        contents: { name: string }[];
        bundle_digest: string | null;
      };
      manifest.contents = manifest.contents.filter(
        (item) => item.name !== "artifact-comp-2",
      );
      const { bundle_digest: _omit, ...rest } = manifest;
      const digest = await contentDigest(
        rest as unknown as Record<string, unknown>,
      );
      manifest.bundle_digest = (
        digest as Digest & { asString(): string }
      ).asString();
      writeFileSync(manifestPath, JSON.stringify(manifest));

      await expectBundleError(
        () =>
          installBundleOffline({
            bundleDir,
            installRoot: join(fixture.base, "install"),
            stagingRoot: join(fixture.base, "staging"),
            backupRoot: join(fixture.base, "backup"),
            quarantineRoot: join(fixture.base, "quarantine"),
            journalRoot: join(fixture.base, "journal"),
            releaseId: RUN.releaseId,
            installId: RUN.installId,
            runId: RUN.runId,
            gitCommit: RUN.gitCommit,
            componentPaths: {
              "comp-1": "bin/nexus-core",
              "comp-2": "models/nexus-model",
            },
          }),
        "BUNDLE_MISSING_FILE",
      );
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_offline_install_unverified_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("unverified");
    try {
      // Tamper a payload: the install re-verifies the bundle and must
      // deny before any mutation.
      writeFileSync(join(bundleDir, "images", "artifact-comp-1"), "tampered");
      await expectBundleError(
        () =>
          installBundleOffline({
            bundleDir,
            installRoot: join(fixture.base, "install"),
            stagingRoot: join(fixture.base, "staging"),
            backupRoot: join(fixture.base, "backup"),
            quarantineRoot: join(fixture.base, "quarantine"),
            journalRoot: join(fixture.base, "journal"),
            releaseId: RUN.releaseId,
            installId: RUN.installId,
            runId: RUN.runId,
            gitCommit: RUN.gitCommit,
            componentPaths: {
              "comp-1": "bin/nexus-core",
              "comp-2": "models/nexus-model",
            },
          }),
        "BUNDLE_DIGEST_MISMATCH",
      );
      // No partial install state may exist.
      expect(existsSync(join(fixture.base, "install"))).toBe(false);
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_rollback_drill_restores_prior", async () => {
    const { fixture, bundleDir } = await makeBundle("drill");
    try {
      const installRoot = join(fixture.base, "install");
      mkdirSync(installRoot, { recursive: true });
      const priorPath = join(installRoot, "prior-state");
      writeFileSync(priorPath, "prior-state-bytes");

      const result = await installBundleOffline({
        bundleDir,
        installRoot,
        stagingRoot: join(fixture.base, "staging"),
        backupRoot: join(fixture.base, "backup"),
        quarantineRoot: join(fixture.base, "quarantine"),
        journalRoot: join(fixture.base, "journal"),
        releaseId: RUN.releaseId,
        installId: RUN.installId,
        runId: RUN.runId,
        gitCommit: RUN.gitCommit,
        componentPaths: {
          "comp-1": "bin/nexus-core",
          "comp-2": "models/nexus-model",
        },
      });
      // New state verified present.
      expect(existsSync(join(installRoot, "bin", "nexus-core"))).toBe(true);
      expect(result.install.backup).toBeDefined();

      const record = await runRollbackDrill({
        installRoot,
        stagingRoot: join(fixture.base, "staging"),
        backupRoot: join(fixture.base, "backup"),
        quarantineRoot: join(fixture.base, "quarantine"),
        journalRoot: join(fixture.base, "journal"),
        releaseId: RUN.releaseId,
        installId: RUN.installId,
        runId: RUN.runId,
        gitCommit: RUN.gitCommit,
        expectedBackupDigest: result.install.backup!.digest,
        expectedPriorBytes: { [priorPath]: "prior-state-bytes" },
      });
      expect(record.receipt_after_verified_restoration).toBe(true);
      expect(record.prior_state_verified).toBe(true);
      // EXACT prior bytes restored; new state gone.
      expect(readFileSync(priorPath, "utf8")).toBe("prior-state-bytes");
      expect(existsSync(join(installRoot, "bin", "nexus-core"))).toBe(false);
      // Receipt exists only after verified restoration.
      expect(
        JSON.parse(
          readFileSync(join(installRoot, ".rollback-receipt.json"), "utf8"),
        ),
      ).toMatchObject({ receipt_after_verified_restoration: true });
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_rollback_drill_wrong_backup_denied", async () => {
    const { fixture, bundleDir } = await makeBundle("wrongbackup");
    try {
      const installRoot = join(fixture.base, "install");
      mkdirSync(installRoot, { recursive: true });
      const priorPath = join(installRoot, "prior-state");
      writeFileSync(priorPath, "prior-state-bytes");

      const result = await installBundleOffline({
        bundleDir,
        installRoot,
        stagingRoot: join(fixture.base, "staging"),
        backupRoot: join(fixture.base, "backup"),
        quarantineRoot: join(fixture.base, "quarantine"),
        journalRoot: join(fixture.base, "journal"),
        releaseId: RUN.releaseId,
        installId: RUN.installId,
        runId: RUN.runId,
        gitCommit: RUN.gitCommit,
        componentPaths: {
          "comp-1": "bin/nexus-core",
          "comp-2": "models/nexus-model",
        },
      });

      // Wrong backup digest -> M4 rollback surface denies.
      await expectBundleError(
        () =>
          runRollbackDrill({
            installRoot,
            stagingRoot: join(fixture.base, "staging"),
            backupRoot: join(fixture.base, "backup"),
            quarantineRoot: join(fixture.base, "quarantine"),
            journalRoot: join(fixture.base, "journal"),
            releaseId: RUN.releaseId,
            installId: RUN.installId,
            runId: RUN.runId,
            gitCommit: RUN.gitCommit,
            expectedBackupDigest:
              "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            expectedPriorBytes: { [priorPath]: "prior-state-bytes" },
          }),
        "ROLLBACK_FAILED",
      );
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_evidence_bound_redacted", async () => {
    const fixture = await makeFixture("evidence");
    try {
      const canary = `sk-ep042-m5-canary-${Date.now()}`;
      const evidence = await buildBundleEvidence({
        runId: RUN.runId,
        gitCommit: RUN.gitCommit,
        releaseId: "release-1",
        installId: "install-1",
        bundleId: "bundle-evidence",
        manifestDigest: "sha256:" + "a".repeat(64),
        bundleDigest: "sha256:" + "b".repeat(64),
        componentDigests: ["sha256:" + "c".repeat(64)],
        bundleVerificationState: "VERIFIED",
        installState: "INSTALLED",
        rollbackState: "VERIFIED",
        offlineInstallState: "OFFLINE_INSTALL_VERIFIED",
        signatureState: "SIGNATURE_PRESENT_NOT_VERIFIED",
        certificationBoundary: [`canary boundary ${canary}`],
        timestamp: new Date().toISOString(),
        secretCanaries: [canary],
      });
      expect(evidence.redaction_result).toBe("REDACTED");
      expect(JSON.stringify(evidence)).not.toContain(canary);
      await validateEvidence(evidence, {
        runId: RUN.runId,
        gitCommit: RUN.gitCommit,
      });
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_evidence_stale_rejected", async () => {
    const fixture = await makeFixture("stale");
    try {
      const evidence = await buildBundleEvidence({
        runId: "ep042-m5-stale-run",
        gitCommit: RUN.gitCommit,
        releaseId: "release-1",
        installId: "install-1",
        bundleId: "bundle-stale",
        manifestDigest: "sha256:" + "a".repeat(64),
        bundleDigest: "sha256:" + "b".repeat(64),
        componentDigests: [],
        bundleVerificationState: "VERIFIED",
        installState: "INSTALLED",
        rollbackState: "VERIFIED",
        offlineInstallState: "OFFLINE_INSTALL_VERIFIED",
        signatureState: "SIGNATURE_PRESENT_NOT_VERIFIED",
        certificationBoundary: [],
        timestamp: new Date().toISOString(),
      });
      await expectBundleError(
        () =>
          validateEvidence(evidence, {
            runId: RUN.runId,
            gitCommit: RUN.gitCommit,
          }),
        "EVIDENCE_INVALID",
      );
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });

  it("ep042_bundle_evidence_tampered_rejected", async () => {
    const fixture = await makeFixture("tampered");
    try {
      const evidence = await buildBundleEvidence({
        runId: RUN.runId,
        gitCommit: RUN.gitCommit,
        releaseId: "release-1",
        installId: "install-1",
        bundleId: "bundle-tampered",
        manifestDigest: "sha256:" + "a".repeat(64),
        bundleDigest: "sha256:" + "b".repeat(64),
        componentDigests: [],
        bundleVerificationState: "VERIFIED",
        installState: "INSTALLED",
        rollbackState: "VERIFIED",
        offlineInstallState: "OFFLINE_INSTALL_VERIFIED",
        signatureState: "SIGNATURE_PRESENT_NOT_VERIFIED",
        certificationBoundary: [],
        timestamp: new Date().toISOString(),
      });
      const tampered = { ...evidence, install_state: "NOT_INSTALLED" };
      await expectBundleError(
        () =>
          validateEvidence(tampered, {
            runId: RUN.runId,
            gitCommit: RUN.gitCommit,
          }),
        "EVIDENCE_INVALID",
      );
    } finally {
      rmSync(fixture.base, { recursive: true, force: true });
    }
  });
});
