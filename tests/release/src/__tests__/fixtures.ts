/**
 * EP-042 M2 test fixtures: canonical wire-shaped objects built in the
 * exact snake_case form of the M1 Rust serde surface. Local test
 * fixtures only - never production data.
 */

import {
  contentDigest,
  manifestContentDigest,
  parseReleaseManifest,
  parseUpdatePlan,
  type BackupProof,
  type CanaryRing,
  type ManualPromotion,
  type OfflineBundle,
  type ReleaseManifest,
  type RollbackDrillEvidence,
  type RollbackReceipt,
  type UpdatePlan,
} from "@nexus/setup";

/** Deterministic 64-char lowercase hex digest. */
export function hex64(seed: string): string {
  let s = "";
  let i = 0;
  while (s.length < 64) {
    const c = seed.charCodeAt(i % seed.length) + i;
    s += (c % 16).toString(16);
    i += 1;
  }
  return s;
}

export function digest(seed: string): string {
  return `sha256:${hex64(seed)}`;
}

export function objectRef(seed: string): { backend: string; key: string } {
  return { backend: "local", key: seed };
}

export function signatureWire(): {
  algorithm: string;
  key_id: string;
  value_b64: string;
} {
  return {
    algorithm: "ED25519",
    key_id: "key-test-1",
    value_b64: "AAAA01BBBB01",
  };
}

export function componentWire(
  id: string,
  version: string,
): Record<string, unknown> {
  return {
    component_id: id,
    name: `component-${id}`,
    version,
    artifact_ref: objectRef(`artifact-${id}`),
    digest: digest(id),
    signature: signatureWire(),
    sbom_ref: objectRef(`sbom-${id}`),
    license_ref: "MIT",
    size_bytes: 1024,
  };
}

export function matrixWire(): Record<string, unknown> {
  return {
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
  };
}

export function manifestWire(): Record<string, unknown> {
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
    sbom_ref: objectRef("sbom-root"),
    license_refs: ["MIT"],
    created_at: "2026-08-25T00:00:00Z",
  };
}

/** Parse the fixture manifest. */
export function fixtureManifest(): ReleaseManifest {
  return parseReleaseManifest(manifestWire());
}

/** Bound manifest: manifest_digest binds real content digest. */
export async function boundManifest(): Promise<ReleaseManifest> {
  const manifest = fixtureManifest();
  const digest = await manifestContentDigest(manifest);
  return parseReleaseManifest({
    ...manifestWire(),
    manifest_digest: digest.asString(),
  });
}

export function planWire(): Record<string, unknown> {
  return {
    schema_version: 1,
    plan_id: "plan-1",
    release_id: "release-1",
    from_version: "1.0.0",
    to_version: "1.1.0",
    channel: "STABLE",
    steps: [
      { order: 1, kind: "BACKUP", description: "backup state before update" },
      { order: 2, kind: "MIGRATE", description: "apply compatible migrations" },
      { order: 3, kind: "CANARY", description: "canary cohort" },
      { order: 4, kind: "OBSERVE", description: "observe health" },
      {
        order: 5,
        kind: "ROLLBACK",
        description: "declared rollback contingency",
      },
    ],
    idempotency_key: "idem-1",
    correlation_id: "corr-1",
    created_at: "2026-08-25T00:00:00Z",
    state: "PLANNED",
  };
}

export function ringWire(): Record<string, unknown> {
  return {
    schema_version: 1,
    ring_id: "ring-1",
    release_id: "release-1",
    profile: "MANAGED",
    cohort_percent: 5,
    observation_minutes: 30,
    health_criterion: "healthz healthy and readyz true",
    verdict: "OBSERVING",
  };
}

export function receiptWire(): Record<string, unknown> {
  return {
    schema_version: 1,
    receipt_id: "receipt-1",
    update_plan_ref: "plan-1",
    from_version: "1.1.0",
    to_version: "1.0.0",
    backup_ref: objectRef("backup-snapshot-1"),
    backup_verification: "VERIFIED",
    rollback_verification: "VERIFIED",
    state: "ROLLBACK_VERIFIED",
    actor: "operator-1",
    correlation_id: "corr-1",
    verified_at: "2026-08-25T01:00:00Z",
  };
}

export function backupProofWire(): BackupProof {
  return {
    backup_id: "backup-1",
    install_id: "install-1",
    digest: digest("backup-1"),
    completed_at: "2026-08-25T00:30:00Z",
    state: "VERIFIED",
  };
}

export function drillWire(): RollbackDrillEvidence {
  return {
    drill_id: "drill-1",
    install_id: "backup-snapshot-1",
    from_version: "1.1.0",
    to_version: "1.0.0",
    verified_at: "2026-08-25T01:15:00Z",
    outcome: "VERIFIED",
  };
}

export function offlineBundleWire(): Record<string, unknown> {
  return {
    schema_version: 1,
    bundle_id: "bundle-1",
    release_id: "release-1",
    contents: [
      { kind: "IMAGE", name: "control-plane", digest: digest("img") },
      { kind: "MODEL", name: "microbrain", digest: digest("model") },
      { kind: "LICENSE", name: "LICENSES", digest: digest("lic") },
      { kind: "SBOM", name: "sbom.json", digest: digest("sbom") },
      { kind: "MIGRATION", name: "migrations", digest: digest("mig") },
      { kind: "RECOVERY_TOOL", name: "recover", digest: digest("rec") },
    ],
    manifest_ref: objectRef("manifest.json"),
    sbom_refs: ["sbom-root"],
    license_refs: ["MIT"],
    migrations: ["migration-001"],
  };
}

export function promotionWire(): Record<string, unknown> {
  return {
    schema_version: 1,
    promotion_id: "promo-1",
    release_id: "release-1",
    update_plan_ref: "plan-1",
    canary_ring_ref: "ring-1",
    approval_ref: "approval-42",
    approver: "operator-1",
    approved_at: "2026-08-25T02:00:00Z",
    state: "APPROVED_MANUAL_ONLY",
    exact_manual_command: "sh scripts/deploy.sh --dry-run --release 1.1.0",
  };
}

export function validPlan(): UpdatePlan {
  return parseUpdatePlan(planWire());
}

export async function manifestContentDigestHex(
  manifest: ReleaseManifest,
): Promise<string> {
  return (await manifestContentDigest(manifest)).asString();
}

export async function contentDigestHex(
  value: Record<string, unknown>,
): Promise<string> {
  return (await contentDigest(value)).asString();
}
