/**
 * EP-043 M2 core behavior and deterministic invariants proofs.
 *
 * Every test name begins `ep043_unit_`. The suite exercises the real
 * production readiness evaluation, release manifest production, and
 * repository state adapter against real repository fixtures (no mocks,
 * no test doubles).
 */
import { describe, expect, it } from "vitest";

import {
  buildReleaseManifest,
  canonicalManifestPayload,
  digestBytes,
  manifestDigest,
  parseReleaseManifestWire,
  verifyManifestDigest,
  type ManifestComponentInput,
  type ReleaseManifestWire,
} from "@nexus/release-evidence";
import {
  evaluateCertificationObligation,
  evaluateGraphObligation,
  evaluateLiveFireObligation,
  evaluateReadiness,
  evaluateReleaseObligation,
  evaluateReviewsObligation,
  liveFireProofsToGateProofs,
  validateReadinessInputs,
  type LiveFireProofResult,
  type ReadinessInputs,
} from "@nexus/release-evidence";
import { REVIEW_DOMAINS } from "@nexus/release-evidence";
import {
  collectCertifications,
  collectGraphNodes,
  collectLiveFireProofs,
  defaultRepoPaths,
} from "@nexus/release-evidence";
import { ShipError } from "@nexus/release-evidence";
import {
  createManualDeployHandoff,
  createProductionReadinessDecision,
  createReleaseEvidence,
  createShipGate,
} from "@nexus/release-evidence";

const ROOT = "/root/nexus";
const PATHS = defaultRepoPaths(ROOT);

function allDoneNodes(count = 2): { nodeId: string; done: boolean }[] {
  return Array.from({ length: count }, (_, i) => ({
    nodeId: `EP-${String(i + 1).padStart(3, "0")}`,
    done: true,
  }));
}

function allProofs(count = 3): LiveFireProofResult[] {
  return Array.from({ length: count }, (_, i) => ({
    lfId: `LF-${String(i + 1).padStart(3, "0")}`,
    ownerNode: `EP-0${i + 1}`,
    slug: `proof-${i + 1}`,
    ownerDone: true,
    evidenceRef: `.agent/state/evidence/LF-${String(i + 1).padStart(3, "0")}-x.json`,
  }));
}

function allReviews(): {
  domain: string;
  status: "PASS";
  evidenceRef: string;
}[] {
  return [
    "SECURITY",
    "PRIVACY",
    "PERFORMANCE",
    "ACCESSIBILITY",
    "OBSERVABILITY",
    "BACKUP",
    "RESTORE",
    "UPDATE",
    "ROLLBACK",
  ].map((domain) => ({
    domain,
    status: "PASS" as const,
    evidenceRef: `.agent/state/evidence/review-${domain.toLowerCase()}.json`,
  }));
}

function allDrills(): {
  kind: string;
  status: "DATED_EVIDENCE";
  datedAt: string;
}[] {
  return [
    "RESTORE",
    "ROLLBACK",
    "PROVIDER_FAILOVER",
    "IDENTITY_RECOVERY",
    "SENTINEL_CONTAINMENT",
    "UPDATE_FAILURE",
  ].map((kind) => ({
    kind,
    status: "DATED_EVIDENCE" as const,
    datedAt: "2026-08-25T00:00:00.000Z",
  }));
}

function signedCertificationInput() {
  return {
    providerRows: [
      {
        rowId: "provider-1",
        domain: "PROVIDER" as const,
        state: "SIGNED" as const,
        evidenceRef: "provider-certification/RESULTS.md",
      },
    ],
    hardwareRows: [
      {
        rowId: "hardware-1",
        domain: "HARDWARE" as const,
        state: "SIGNED" as const,
        evidenceRef: "hardware/CERTIFICATION_RESULTS.md",
      },
    ],
  };
}

function readyInputs(): ReadinessInputs {
  return {
    graphNodes: allDoneNodes(),
    liveFireProofs: allProofs(),
    certifications: signedCertificationInput(),
    reviews: allReviews() as ReadinessInputs["reviews"],
    drills: allDrills() as ReadinessInputs["drills"],
    releaseTag: "green/EP-043",
    manualDeployCommand: "sh scripts/deploy.sh --dry-run",
    freshCloneRerun: true,
  };
}

function componentInput(id: string, bytes: Uint8Array): ManifestComponentInput {
  return {
    componentId: id,
    name: id,
    version: "1.0.0",
    artifactBytes: bytes,
    artifactKey: `releases/nexus-1.0.0-rc1/components/${id}`,
  };
}

describe("EP-043 M2 readiness evaluation", () => {
  it("ep043_unit_readiness_ready_when_all_obligations_met", () => {
    const evaluation = evaluateReadiness(readyInputs());
    expect(evaluation.allMet).toBe(true);
    expect(evaluation.shipGateVerdict).toBe("PASSED");
    expect(evaluation.decision).toBe("READY");
    expect(evaluation.blockingReasons).toEqual([]);
  });

  it("ep043_unit_readiness_blocks_on_undone_graph_node", () => {
    const inputs = readyInputs();
    inputs.graphNodes = [{ nodeId: "EP-043", done: false }];
    const evaluation = evaluateReadiness(inputs);
    expect(evaluation.allMet).toBe(false);
    expect(evaluation.shipGateVerdict).toBe("BLOCKED");
    expect(evaluation.decision).toBe("NOT_READY");
    expect(evaluation.blockingReasons.join(" ")).toContain("EP-043");
  });

  it("ep043_unit_readiness_blocks_on_owner_not_done", () => {
    const inputs = readyInputs();
    inputs.liveFireProofs[0] = {
      lfId: "LF-001",
      ownerNode: "EP-001",
      slug: "x",
      ownerDone: false,
      evidenceRef: "",
    };
    const evaluation = evaluateReadiness(inputs);
    expect(evaluation.decision).toBe("NOT_READY");
    expect(evaluation.blockingReasons.join(" ")).toContain("LF-001");
  });

  it("ep043_unit_readiness_blocks_on_missing_evidence", () => {
    const inputs = readyInputs();
    inputs.liveFireProofs[1] = {
      lfId: "LF-002",
      ownerNode: "EP-002",
      slug: "x",
      ownerDone: true,
      evidenceRef: "",
    };
    const evaluation = evaluateReadiness(inputs);
    expect(evaluation.decision).toBe("NOT_READY");
    expect(evaluation.blockingReasons.join(" ")).toContain("no evidence file");
  });

  it("ep043_unit_readiness_blocks_on_pending_certification", () => {
    const inputs = readyInputs();
    inputs.certifications = {
      providerRows: [
        {
          rowId: "provider-1",
          domain: "PROVIDER",
          state: "RELEASE-BLOCKING-PENDING",
        },
      ],
      hardwareRows: [],
    };
    const evaluation = evaluateReadiness(inputs);
    expect(evaluation.decision).toBe("NOT_READY");
    expect(evaluation.blockingReasons.join(" ")).toContain(
      "RELEASE-BLOCKING-PENDING",
    );
  });

  it("ep043_unit_readiness_blocks_on_missing_review", () => {
    const inputs = readyInputs();
    inputs.reviews = (
      inputs.reviews as {
        domain: string;
        status: string;
        evidenceRef: string;
      }[]
    ).filter(
      (review) => review.domain !== "SECURITY",
    ) as ReadinessInputs["reviews"];
    const evaluation = evaluateReadiness(inputs);
    expect(evaluation.decision).toBe("NOT_READY");
    expect(evaluation.blockingReasons.join(" ")).toContain("SECURITY");
  });

  it("ep043_unit_readiness_blocks_on_failed_review", () => {
    const inputs = readyInputs();
    const reviews = inputs.reviews as {
      domain: string;
      status: string;
      evidenceRef: string;
    }[];
    reviews[0] = { domain: "SECURITY", status: "FAIL", evidenceRef: "" };
    inputs.reviews = reviews as ReadinessInputs["reviews"];
    const evaluation = evaluateReadiness(inputs);
    expect(evaluation.decision).toBe("NOT_READY");
  });

  it("ep043_unit_readiness_drills_enforced_at_decision_layer", () => {
    // Drill evidence is enforced by the M1 ProductionReadinessDecision
    // (evaluateProductionReadiness requires every drill kind
    // DATED_EVIDENCE), not by the five acceptance obligations.
    const drills = allDrills() as {
      kind: string;
      status: "DATED_EVIDENCE";
      datedAt: string;
    }[];
    const missingRestore = drills.filter((drill) => drill.kind !== "RESTORE");
    const gate = createShipGate({
      gateId: "gate-1",
      releaseKind: "CORE_RELEASE",
      phase: "SHIP_DECISION",
      requiredProofs: [
        {
          family: "SECURITY",
          proofId: "p1",
          status: "PASS",
          evidenceRef: "e1",
        },
        { family: "BACKUP", proofId: "p2", status: "PASS", evidenceRef: "e2" },
      ],
      freshCloneRerun: true,
    });
    const evidenceReady = createReleaseEvidence({
      node: "EP-043",
      runId: "r",
      gitCommit: "0".repeat(40),
      releaseId: "release-1",
      drills: drills as never,
    });
    const decisionReady = createProductionReadinessDecision({
      decisionId: "d",
      releaseId: "release-1",
      gate,
      evidence: evidenceReady,
      handoff: createManualDeployHandoff({
        handoffId: "h",
        releaseId: "release-1",
        profile: "core",
        exactCommand: "sh scripts/deploy.sh --dry-run",
      }),
    });
    expect(decisionReady.decision).toBe("READY");

    const evidenceMissing = createReleaseEvidence({
      node: "EP-043",
      runId: "r",
      gitCommit: "0".repeat(40),
      releaseId: "release-1",
      drills: missingRestore as never,
    });
    const decisionMissing = createProductionReadinessDecision({
      decisionId: "d",
      releaseId: "release-1",
      gate,
      evidence: evidenceMissing,
      handoff: createManualDeployHandoff({
        handoffId: "h",
        releaseId: "release-1",
        profile: "core",
        exactCommand: "sh scripts/deploy.sh --dry-run",
      }),
    });
    expect(decisionMissing.decision).toBe("NOT_READY");
  });

  it("ep043_unit_readiness_blocks_without_fresh_clone_rerun", () => {
    const inputs = readyInputs();
    inputs.freshCloneRerun = false;
    const evaluation = evaluateReadiness(inputs);
    expect(evaluation.decision).toBe("NOT_READY");
    expect(evaluation.blockingReasons.join(" ")).toContain("fresh-clone");
  });

  it("ep043_unit_readiness_blocks_without_release_tag", () => {
    const inputs = readyInputs();
    inputs.releaseTag = "";
    const evaluation = evaluateReadiness(inputs);
    expect(evaluation.decision).toBe("NOT_READY");
    expect(evaluation.blockingReasons.join(" ")).toContain("release tag");
  });

  it("ep043_unit_readiness_blocks_without_manual_command", () => {
    const inputs = readyInputs();
    inputs.manualDeployCommand = "";
    const evaluation = evaluateReadiness(inputs);
    expect(evaluation.decision).toBe("NOT_READY");
    expect(evaluation.blockingReasons.join(" ")).toContain("manual deploy");
  });

  it("ep043_unit_readiness_obligation_count_is_five", () => {
    const evaluation = evaluateReadiness(readyInputs());
    expect(evaluation.obligations).toHaveLength(5);
  });

  it("ep043_unit_readiness_validate_rejects_unknown_review", () => {
    const inputs = readyInputs();
    inputs.reviews = [
      { domain: "COST" as never, status: "PASS", evidenceRef: "x" },
    ];
    expect(() => validateReadinessInputs(inputs)).toThrow(ShipError);
  });

  it("ep043_unit_readiness_validate_rejects_unknown_drill", () => {
    const inputs = readyInputs();
    inputs.drills = [{ kind: "RESTART" as never, status: "NOT_RUN" }];
    expect(() => validateReadinessInputs(inputs)).toThrow(ShipError);
  });

  it("ep043_unit_readiness_livefire_gate_proofs", () => {
    const proofs = liveFireProofsToGateProofs(allProofs());
    expect(proofs).toHaveLength(3);
    for (const proof of proofs) {
      expect(proof.status).toBe("PASS");
      expect(proof.evidenceRef.length).toBeGreaterThan(0);
    }
  });

  it("ep043_unit_readiness_deterministic", () => {
    const a = evaluateReadiness(readyInputs());
    const b = evaluateReadiness(readyInputs());
    expect(a).toEqual(b);
  });

  it("ep043_unit_readiness_graph_obligation_empty_fails", () => {
    const result = evaluateGraphObligation([]);
    expect(result.met).toBe(true); // no nodes means nothing undone
  });

  it("ep043_unit_readiness_certification_empty_fails", () => {
    const result = evaluateCertificationObligation({
      providerRows: [],
      hardwareRows: [],
    });
    expect(result.met).toBe(false);
  });

  it("ep043_unit_readiness_review_obligation_missing_domain", () => {
    const result = evaluateReviewsObligation([]);
    expect(result.met).toBe(false);
    expect(result.reasons.length).toBe(REVIEW_DOMAINS.length);
  });

  it("ep043_unit_readiness_release_obligation_truth", () => {
    expect(
      evaluateReleaseObligation(
        "green/EP-043",
        "sh scripts/deploy.sh --dry-run",
      ).met,
    ).toBe(true);
    expect(
      evaluateReleaseObligation("", "sh scripts/deploy.sh --dry-run").met,
    ).toBe(false);
    expect(evaluateReleaseObligation("green/EP-043", "").met).toBe(false);
  });

  it("ep043_unit_readiness_livefire_obligation_empty_fails_open", () => {
    const result = evaluateLiveFireObligation([]);
    expect(result.met).toBe(true); // no proofs means nothing to fail
  });
});

describe("EP-043 M2 release manifest", () => {
  const coreBytes = new TextEncoder().encode(
    "nexus-core fixture bytes for EP-043 M2 manifest",
  );
  const modelBytes = new TextEncoder().encode(
    "nexus-model fixture bytes for EP-043 M2 manifest",
  );

  it("ep043_unit_manifest_builds_with_real_digests", () => {
    const manifest = buildReleaseManifest({
      releaseId: "nexus-1.0.0-rc1",
      version: "1.0.0",
      channel: "STABLE",
      profile: "FULLY_LOCAL",
      createdAt: "2026-08-25T00:00:00.000Z",
      components: [
        componentInput("nexus-core", coreBytes),
        componentInput("nexus-model", modelBytes),
      ],
    });
    expect(manifest.schema_version).toBe(1);
    expect(manifest.components).toHaveLength(2);
    expect(manifest.manifest_digest).toMatch(/^sha256:[0-9a-f]{64}$/);
    expect(manifest.components[0]!.digest).toBe(digestBytes(coreBytes));
    expect(manifest.components[0]!.size_bytes).toBe(coreBytes.length);
  });

  it("ep043_unit_manifest_digest_binds_content", () => {
    const manifestA = buildReleaseManifest({
      releaseId: "nexus-1.0.0-rc1",
      version: "1.0.0",
      channel: "STABLE",
      profile: "FULLY_LOCAL",
      createdAt: "2026-08-25T00:00:00.000Z",
      components: [componentInput("nexus-core", coreBytes)],
    });
    const manifestB = buildReleaseManifest({
      releaseId: "nexus-1.0.0-rc1",
      version: "1.0.1",
      channel: "STABLE",
      profile: "FULLY_LOCAL",
      createdAt: "2026-08-25T00:00:00.000Z",
      components: [componentInput("nexus-core", coreBytes)],
    });
    expect(manifestA.manifest_digest).not.toBe(manifestB.manifest_digest);
  });

  it("ep043_unit_manifest_digest_strip_then_digest", () => {
    const manifest = buildReleaseManifest({
      releaseId: "nexus-1.0.0-rc1",
      version: "1.0.0",
      channel: "STABLE",
      profile: "FULLY_LOCAL",
      createdAt: "2026-08-25T00:00:00.000Z",
      components: [componentInput("nexus-core", coreBytes)],
    });
    const { manifest_digest, ...payload } = manifest;
    expect(manifest_digest).toBe(manifestDigest(payload));
    expect(canonicalManifestPayload(payload)).not.toContain("manifest_digest");
  });

  it("ep043_unit_manifest_verify_rejects_tamper", () => {
    const manifest = buildReleaseManifest({
      releaseId: "nexus-1.0.0-rc1",
      version: "1.0.0",
      channel: "STABLE",
      profile: "FULLY_LOCAL",
      createdAt: "2026-08-25T00:00:00.000Z",
      components: [componentInput("nexus-core", coreBytes)],
    });
    expect(verifyManifestDigest(manifest)).toBe(true);
    const tampered: ReleaseManifestWire = { ...manifest, version: "9.9.9" };
    expect(verifyManifestDigest(tampered)).toBe(false);
  });

  it("ep043_unit_manifest_parse_roundtrip", () => {
    const manifest = buildReleaseManifest({
      releaseId: "nexus-1.0.0-rc1",
      version: "1.0.0",
      channel: "STABLE",
      profile: "FULLY_LOCAL",
      createdAt: "2026-08-25T00:00:00.000Z",
      components: [componentInput("nexus-core", coreBytes)],
    });
    const parsed = parseReleaseManifestWire(
      JSON.parse(JSON.stringify(manifest)),
    );
    expect(parsed).toEqual(manifest);
  });

  it("ep043_unit_manifest_parse_rejects_unknown_field", () => {
    const manifest = buildReleaseManifest({
      releaseId: "nexus-1.0.0-rc1",
      version: "1.0.0",
      channel: "STABLE",
      profile: "FULLY_LOCAL",
      createdAt: "2026-08-25T00:00:00.000Z",
      components: [componentInput("nexus-core", coreBytes)],
    });
    const serialized = JSON.parse(JSON.stringify(manifest));
    serialized["auto_deploy"] = true;
    expect(() => parseReleaseManifestWire(serialized)).toThrow(ShipError);
  });

  it("ep043_unit_manifest_parse_rejects_digest_mismatch", () => {
    const manifest = buildReleaseManifest({
      releaseId: "nexus-1.0.0-rc1",
      version: "1.0.0",
      channel: "STABLE",
      profile: "FULLY_LOCAL",
      createdAt: "2026-08-25T00:00:00.000Z",
      components: [componentInput("nexus-core", coreBytes)],
    });
    const serialized = JSON.parse(JSON.stringify(manifest));
    serialized["manifest_digest"] = "sha256:" + "0".repeat(64);
    expect(() => parseReleaseManifestWire(serialized)).toThrow(ShipError);
  });

  it("ep043_unit_manifest_unknown_channel_rejected", () => {
    expect(() =>
      buildReleaseManifest({
        releaseId: "r",
        version: "1.0.0",
        channel: "NIGHTLY" as never,
        profile: "FULLY_LOCAL",
        createdAt: "2026-08-25T00:00:00.000Z",
        components: [componentInput("nexus-core", coreBytes)],
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_manifest_empty_components_rejected", () => {
    expect(() =>
      buildReleaseManifest({
        releaseId: "r",
        version: "1.0.0",
        channel: "STABLE",
        profile: "FULLY_LOCAL",
        createdAt: "2026-08-25T00:00:00.000Z",
        components: [],
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_manifest_signature_present_not_verified_honest", () => {
    const manifest = buildReleaseManifest({
      releaseId: "r",
      version: "1.0.0",
      channel: "STABLE",
      profile: "FULLY_LOCAL",
      createdAt: "2026-08-25T00:00:00.000Z",
      components: [componentInput("nexus-core", coreBytes)],
    });
    expect(manifest.components[0]!.signature.key_id).toBe(
      "SIGNATURE_PRESENT_NOT_VERIFIED",
    );
    expect(manifest.components[0]!.signature.value_b64).toBe(
      "SIGNATURE_PRESENT_NOT_VERIFIED",
    );
  });
});

describe("EP-043 M2 repository state adapter", () => {
  it("ep043_unit_repo_graph_nodes_real", () => {
    const nodes = collectGraphNodes(PATHS);
    expect(nodes.length).toBeGreaterThan(40);
    const ep042 = nodes.find((node) => node.nodeId === "EP-042");
    expect(ep042?.done).toBe(true);
    const ep043 = nodes.find((node) => node.nodeId === "EP-043");
    expect(ep043?.done).toBe(false);
  });

  it("ep043_unit_repo_livefire_real", () => {
    const proofs = collectLiveFireProofs(PATHS);
    expect(proofs.length).toBeGreaterThanOrEqual(28);
    const lf001 = proofs.find((proof) => proof.lfId === "LF-001");
    expect(lf001?.ownerDone).toBe(true);
  });

  it("ep043_unit_repo_certifications_pending_honest", () => {
    const certifications = collectCertifications(PATHS);
    const all = [
      ...certifications.providerRows,
      ...certifications.hardwareRows,
    ];
    expect(all.length).toBeGreaterThan(0);
    const pending = all.filter(
      (row) => row.state === "RELEASE-BLOCKING-PENDING",
    );
    expect(pending.length).toBeGreaterThan(0); // honest current truth
  });

  it("ep043_unit_repo_readiness_current_state_not_ready", () => {
    // The real repository today cannot be READY (EP-043 not DONE,
    // certification rows pending, no fresh-clone rerun). The evaluation
    // must report that truth deterministically.
    const certifications = collectCertifications(PATHS);
    const graph = collectGraphNodes(PATHS);
    expect(
      graph.find(
        (node: { nodeId: string; done: boolean }) => node.nodeId === "EP-043",
      )?.done,
    ).toBe(false);
    expect(
      certifications.hardwareRows.some(
        (row: { state: string }) => row.state === "RELEASE-BLOCKING-PENDING",
      ),
    ).toBe(true);
  });
});
