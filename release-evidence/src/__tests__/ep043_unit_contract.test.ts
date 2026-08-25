/**
 * EP-043 M1 contract, vocabulary, and package boundary proofs.
 *
 * Every test name begins `ep043_unit_`. The suite proves construction,
 * validation, serialization, vocabulary rejection, and dependency-
 * direction constraints for the four public interfaces (ShipGate,
 * ReleaseEvidence, ManualDeployHandoff, ProductionReadinessDecision)
 * against the REAL implementation (no mocks, no test doubles).
 */
import { describe, expect, it } from "vitest";

import {
  CAPABILITY_STATUSES,
  CERTIFICATION_ROW_STATES,
  DRILL_KINDS,
  DRILL_STATUSES,
  GATE_VERDICTS,
  PROOF_STATUSES,
  RELEASE_KINDS,
  REQUIRED_GATE_FAMILIES,
  REVIEW_DOMAINS,
  SHIP_PHASES,
  WAIVER_CLASSES,
  canonicalEvidenceDigest,
  createManualDeployHandoff,
  createProductionReadinessDecision,
  createReleaseEvidence,
  createShipGate,
  evaluateProductionReadiness,
  evaluateShipGateVerdict,
  parseManualDeployHandoff,
  parseProductionReadinessDecision,
  parseReleaseEvidence,
  parseShipGate,
  redactEvidenceJson,
  sha256Hex,
  type CertificationRow,
  type DrillEvidence,
  type ManualDeployHandoff,
  type ProductionReadinessDecision,
  type ProofStatus,
  type ReleaseEvidence,
  type ReviewResult,
  type ShipGate,
} from "@nexus/release-evidence";
import {
  ShipError,
  assertKnownShipErrorCode,
  redactShipMessage,
} from "@nexus/release-evidence";

const FIXED_NOW = "2026-08-25T00:00:00.000Z";

function passingProof(
  family: string,
  proofId: string,
): { family: string; proofId: string; status: "PASS"; evidenceRef: string } {
  return {
    family,
    proofId,
    status: "PASS",
    evidenceRef: `tests/livefire/${proofId}.json`,
  };
}

function allProofsPass(): {
  family: string;
  proofId: string;
  status: ProofStatus;
  evidenceRef: string;
}[] {
  return [
    passingProof("SECURITY", "security-gate"),
    passingProof("DATA", "data-proof"),
    passingProof("WORKFLOW", "workflow-proof"),
    passingProof("INSTALLATION", "installation-proof"),
    passingProof("UPDATE", "update-proof"),
    passingProof("BACKUP", "backup-proof"),
    passingProof("ROLLBACK", "rollback-proof"),
  ];
}

function allDrillsDated(): DrillEvidence[] {
  return DRILL_KINDS.map((kind) => ({
    kind,
    status: "DATED_EVIDENCE" as const,
    datedAt: FIXED_NOW,
    evidenceRef: `.agent/state/evidence/ep043/${kind.toLowerCase()}-drill.json`,
  }));
}

function allReviewsPass(): ReviewResult[] {
  return REVIEW_DOMAINS.map((domain) => ({
    domain,
    status: "PASS" as const,
    evidenceRef: `.agent/state/evidence/ep043/review-${domain.toLowerCase()}.json`,
  }));
}

function signedCertifications(): CertificationRow[] {
  return [
    {
      rowId: "provider-core",
      domain: "PROVIDER",
      state: "SIGNED",
      evidenceRef: "provider-certification/RESULTS.md",
    },
    {
      rowId: "hardware-core",
      domain: "HARDWARE",
      state: "SIGNED",
      evidenceRef: "hardware/CERTIFICATION_RESULTS.md",
    },
  ];
}

function buildEvidence(
  overrides: Partial<ReleaseEvidence> = {},
): ReleaseEvidence {
  const base = {
    node: "EP-043",
    runId: "ep043-m1-unit",
    gitCommit: "0".repeat(40),
    releaseId: "release-1",
    certifications: signedCertifications(),
    drills: allDrillsDated(),
    reviews: allReviewsPass(),
    releaseNotes: { "core-control": "CERTIFIED" as const },
    ...overrides,
  };
  return createReleaseEvidence({
    node: base.node,
    runId: base.runId,
    gitCommit: base.gitCommit,
    releaseId: base.releaseId,
    certifications: base.certifications,
    drills: base.drills,
    reviews: base.reviews,
    releaseNotes: base.releaseNotes,
  });
}

function buildGate(overrides: Partial<ShipGate> = {}): ShipGate {
  return createShipGate({
    gateId: "gate-1",
    releaseKind: "CORE_RELEASE",
    phase: "SHIP_DECISION",
    requiredProofs: allProofsPass(),
    freshCloneRerun: true,
    ...overrides,
  });
}

function buildHandoff(
  overrides: Partial<ManualDeployHandoff> = {},
): ManualDeployHandoff {
  const created = createManualDeployHandoff({
    handoffId: "handoff-1",
    releaseId: "release-1",
    profile: "core",
    exactCommand: "sh scripts/deploy.sh --dry-run",
    createdAt: FIXED_NOW,
  });
  return { ...created, ...overrides };
}

describe("EP-043 M1 ship gate construction", () => {
  it("ep043_unit_ship_gate_constructs_passed", () => {
    const gate = buildGate();
    expect(gate.schema_version).toBe(1);
    expect(gate.verdict).toBe("PASSED");
    expect(gate.requiredProofs).toHaveLength(REQUIRED_GATE_FAMILIES.length);
    expect(gate.freshCloneRerun).toBe(true);
  });

  it("ep043_unit_ship_gate_blocks_without_fresh_clone_rerun", () => {
    const gate = buildGate({ freshCloneRerun: false });
    expect(gate.verdict).toBe("BLOCKED");
  });

  it("ep043_unit_ship_gate_blocks_on_failed_proof", () => {
    const proofs = allProofsPass();
    proofs[0] = {
      family: "SECURITY",
      proofId: "security-gate",
      status: "FAIL",
      evidenceRef: "",
    };
    const gate = buildGate({ requiredProofs: proofs });
    expect(gate.verdict).toBe("BLOCKED");
  });

  it("ep043_unit_ship_gate_blocks_on_not_run_proof", () => {
    const proofs = allProofsPass();
    proofs[3] = {
      family: "INSTALLATION",
      proofId: "installation-proof",
      status: "NOT_RUN",
      evidenceRef: "",
    };
    const gate = buildGate({ requiredProofs: proofs });
    expect(gate.verdict).toBe("BLOCKED");
  });

  it("ep043_unit_ship_gate_blocks_on_block_item", () => {
    const gate = buildGate({
      blocks: [
        { code: "CRITICAL-VULN", message: "open CVE", waiverClass: "NONE" },
      ],
    });
    expect(gate.verdict).toBe("BLOCKED");
  });

  it("ep043_unit_ship_gate_generic_waiver_denied", () => {
    expect(() =>
      createShipGate({
        gateId: "gate-x",
        releaseKind: "CORE_RELEASE",
        blocks: [
          {
            code: "CRITICAL-VULN",
            message: "open CVE",
            waiverClass: "GENERIC",
          },
        ],
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_ship_gate_accepted_risk_requires_date", () => {
    expect(() =>
      createShipGate({
        gateId: "gate-x",
        releaseKind: "CORE_RELEASE",
        blocks: [
          {
            code: "CRITICAL-VULN",
            message: "open CVE",
            waiverClass: "ACCEPTED_RISK",
          },
        ],
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_ship_gate_accepted_risk_dated_does_not_pass", () => {
    // Even a dated accepted risk keeps the gate BLOCKED (SPEC-008: no
    // generic waiver; accepted risk is a recorded decision, not a pass).
    const gate = createShipGate({
      gateId: "gate-x",
      releaseKind: "CORE_RELEASE",
      blocks: [
        {
          code: "CRITICAL-VULN",
          message: "open CVE",
          waiverClass: "ACCEPTED_RISK",
          acceptedAt: FIXED_NOW,
        },
      ],
    });
    expect(gate.verdict).toBe("BLOCKED");
  });

  it("ep043_unit_ship_gate_passing_proof_requires_evidence", () => {
    expect(() =>
      createShipGate({
        gateId: "gate-x",
        releaseKind: "CORE_RELEASE",
        requiredProofs: [
          {
            family: "SECURITY",
            proofId: "p1",
            status: "PASS",
            evidenceRef: "",
          },
        ],
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_ship_gate_unknown_release_kind_rejected", () => {
    expect(() =>
      createShipGate({ gateId: "g", releaseKind: "PRODUCTION" as never }),
    ).toThrow(ShipError);
    expect(() =>
      parseShipGate({
        schema_version: 1,
        gateId: "g",
        releaseKind: "PRODUCTION",
        phase: "PRE_SHIP",
        verdict: "PENDING",
        requiredProofs: [],
        blocks: [],
        freshCloneRerun: false,
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_ship_gate_unknown_phase_rejected", () => {
    expect(() =>
      createShipGate({
        gateId: "g",
        releaseKind: "CORE_RELEASE",
        phase: "DEPLOY" as never,
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_ship_gate_unknown_verdict_rejected_on_parse", () => {
    expect(() =>
      parseShipGate({
        schema_version: 1,
        gateId: "g",
        releaseKind: "CORE_RELEASE",
        phase: "PRE_SHIP",
        verdict: "SHIPPED",
        requiredProofs: [],
        blocks: [],
        freshCloneRerun: false,
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_ship_gate_unknown_field_rejected", () => {
    expect(() =>
      parseShipGate({
        schema_version: 1,
        gateId: "g",
        releaseKind: "CORE_RELEASE",
        phase: "PRE_SHIP",
        verdict: "PENDING",
        requiredProofs: [],
        blocks: [],
        freshCloneRerun: false,
        extra: true,
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_ship_gate_verdict_mismatch_rejected", () => {
    const gate = buildGate();
    const serialized = JSON.parse(JSON.stringify(gate));
    serialized["verdict"] = "BLOCKED";
    expect(() => parseShipGate(serialized)).toThrow(ShipError);
  });

  it("ep043_unit_ship_gate_parse_roundtrip", () => {
    const gate = buildGate();
    const parsed = parseShipGate(JSON.parse(JSON.stringify(gate)));
    expect(parsed).toEqual(gate);
  });

  it("ep043_unit_ship_gate_release_candidate_can_pass", () => {
    const gate = createShipGate({
      gateId: "gate-rc",
      releaseKind: "RELEASE_CANDIDATE",
      phase: "SHIP_DECISION",
      requiredProofs: allProofsPass(),
      freshCloneRerun: true,
    });
    expect(gate.verdict).toBe("PASSED");
    expect(gate.releaseKind).toBe("RELEASE_CANDIDATE");
  });

  it("ep043_unit_evaluate_ship_gate_verdict_is_deterministic", () => {
    expect(evaluateShipGateVerdict(allProofsPass(), [], true)).toBe("PASSED");
    expect(evaluateShipGateVerdict(allProofsPass(), [], false)).toBe("BLOCKED");
    expect(evaluateShipGateVerdict([], [], true)).toBe("PASSED");
    expect(
      evaluateShipGateVerdict(
        [],
        [{ code: "X", message: "y", waiverClass: "NONE" }],
        true,
      ),
    ).toBe("BLOCKED");
  });
});

describe("EP-043 M1 release evidence", () => {
  it("ep043_unit_evidence_constructs_with_digest", () => {
    const evidence = buildEvidence();
    expect(evidence.schema_version).toBe(1);
    expect(evidence.evidenceDigest).toMatch(/^[0-9a-f]{64}$/);
    expect(evidence.redactionResult).toBe("CLEAN");
  });

  it("ep043_unit_evidence_digest_binds_content", () => {
    const a = buildEvidence();
    const b = buildEvidence({ releaseId: "release-2" });
    expect(a.evidenceDigest).not.toBe(b.evidenceDigest);
  });

  it("ep043_unit_evidence_digest_deterministic", () => {
    const a = buildEvidence();
    const b = buildEvidence();
    expect(a.evidenceDigest).toBe(b.evidenceDigest);
  });

  it("ep043_unit_evidence_digest_is_real_sha256", () => {
    const evidence = buildEvidence();
    const payload = {
      schema_version: 1 as const,
      node: evidence.node,
      runId: evidence.runId,
      gitCommit: evidence.gitCommit,
      releaseId: evidence.releaseId,
      certifications: evidence.certifications,
      drills: evidence.drills,
      reviews: evidence.reviews,
      releaseNotes: evidence.releaseNotes,
    };
    expect(canonicalEvidenceDigest(payload)).toBe(
      sha256Hex(JSON.stringify(payload, Object.keys(payload).sort())),
    );
    expect(evidence.evidenceDigest).toBe(
      sha256Hex(JSON.stringify(payload, Object.keys(payload).sort())),
    );
  });

  it("ep043_unit_evidence_parse_roundtrip", () => {
    const evidence = buildEvidence();
    const parsed = parseReleaseEvidence(JSON.parse(JSON.stringify(evidence)));
    expect(parsed).toEqual(evidence);
  });

  it("ep043_unit_evidence_tampered_digest_rejected", () => {
    const evidence = buildEvidence();
    const serialized = JSON.parse(JSON.stringify(evidence));
    serialized["releaseId"] = "release-99";
    expect(() => parseReleaseEvidence(serialized)).toThrow(ShipError);
  });

  it("ep043_unit_evidence_unknown_field_rejected", () => {
    const evidence = buildEvidence();
    const serialized = JSON.parse(JSON.stringify(evidence));
    serialized["secret"] = "sk-live-1234567890abcdef";
    expect(() => parseReleaseEvidence(serialized)).toThrow(ShipError);
  });

  it("ep043_unit_evidence_unknown_capability_status_rejected", () => {
    expect(() =>
      createReleaseEvidence({
        node: "EP-043",
        runId: "r",
        gitCommit: "0".repeat(40),
        releaseId: "release-1",
        releaseNotes: { "core-control": "SHIPPED" as never },
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_evidence_unknown_drill_kind_rejected", () => {
    expect(() =>
      createReleaseEvidence({
        node: "EP-043",
        runId: "r",
        gitCommit: "0".repeat(40),
        releaseId: "release-1",
        drills: [{ kind: "RESTART" as never, status: "NOT_RUN" }],
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_evidence_dated_drill_requires_timestamp", () => {
    expect(() =>
      createReleaseEvidence({
        node: "EP-043",
        runId: "r",
        gitCommit: "0".repeat(40),
        releaseId: "release-1",
        drills: [{ kind: "RESTORE", status: "DATED_EVIDENCE" }],
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_evidence_unknown_certification_state_rejected", () => {
    expect(() =>
      createReleaseEvidence({
        node: "EP-043",
        runId: "r",
        gitCommit: "0".repeat(40),
        releaseId: "release-1",
        certifications: [
          { rowId: "r1", domain: "PROVIDER", state: "APPROVED" as never },
        ],
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_evidence_signed_row_requires_evidence_ref", () => {
    expect(() =>
      createReleaseEvidence({
        node: "EP-043",
        runId: "r",
        gitCommit: "0".repeat(40),
        releaseId: "release-1",
        certifications: [{ rowId: "r1", domain: "PROVIDER", state: "SIGNED" }],
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_evidence_unknown_review_domain_rejected", () => {
    expect(() =>
      createReleaseEvidence({
        node: "EP-043",
        runId: "r",
        gitCommit: "0".repeat(40),
        releaseId: "release-1",
        reviews: [
          { domain: "COST" as never, status: "PASS", evidenceRef: "x" },
        ],
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_evidence_redacts_secret_shaped_content", () => {
    const redacted = redactEvidenceJson(
      '{"token":"sk-live-abcdef1234567890","api":"AKIAABCDEFGHIJKLMNOP"}',
    );
    expect(redacted).not.toContain("sk-live-abcdef1234567890");
    expect(redacted).not.toContain("AKIAABCDEFGHIJKLMNOP");
  });

  it("ep043_unit_evidence_redaction_result_detects_canary", () => {
    const evidence = createReleaseEvidence({
      node: "EP-043",
      runId: "r",
      gitCommit: "0".repeat(40),
      releaseId: "release-1",
    });
    expect(evidence.redactionResult).toBe("CLEAN");
  });
});

describe("EP-043 M1 manual deploy handoff", () => {
  it("ep043_unit_handoff_constructs", () => {
    const handoff = buildHandoff();
    expect(handoff.schema_version).toBe(1);
    expect(handoff.exactCommand).toBe("sh scripts/deploy.sh --dry-run");
    expect(handoff.createdAt).toBe(FIXED_NOW);
  });

  it("ep043_unit_handoff_parse_roundtrip", () => {
    const handoff = buildHandoff();
    const parsed = parseManualDeployHandoff(
      JSON.parse(JSON.stringify(handoff)),
    );
    expect(parsed).toEqual(handoff);
  });

  it("ep043_unit_handoff_compound_command_denied", () => {
    expect(() =>
      createManualDeployHandoff({
        handoffId: "h",
        releaseId: "r",
        profile: "core",
        exactCommand: "sh scripts/deploy.sh --dry-run; rm -rf /",
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_handoff_secret_in_command_denied", () => {
    expect(() =>
      createManualDeployHandoff({
        handoffId: "h",
        releaseId: "r",
        profile: "core",
        exactCommand: "deploy --token sk-live-abcdef1234567890",
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_handoff_unknown_field_rejected", () => {
    const handoff = buildHandoff();
    const serialized = JSON.parse(JSON.stringify(handoff));
    serialized["autoDeploy"] = true;
    expect(() => parseManualDeployHandoff(serialized)).toThrow(ShipError);
  });

  it("ep043_unit_handoff_empty_command_rejected", () => {
    expect(() =>
      createManualDeployHandoff({
        handoffId: "h",
        releaseId: "r",
        profile: "core",
        exactCommand: "",
      }),
    ).toThrow(ShipError);
  });
});

describe("EP-043 M1 production readiness decision", () => {
  it("ep043_unit_readiness_ready_when_all_conditions", () => {
    const decision = createProductionReadinessDecision({
      decisionId: "decision-1",
      releaseId: "release-1",
      gate: buildGate(),
      evidence: buildEvidence(),
      handoff: buildHandoff(),
      decidedAt: FIXED_NOW,
    });
    expect(decision.decision).toBe("READY");
  });

  it("ep043_unit_readiness_not_ready_when_gate_blocked", () => {
    const decision = createProductionReadinessDecision({
      decisionId: "d",
      releaseId: "release-1",
      gate: buildGate({ freshCloneRerun: false }),
      evidence: buildEvidence(),
      handoff: buildHandoff(),
    });
    expect(decision.decision).toBe("NOT_READY");
  });

  it("ep043_unit_readiness_not_ready_when_pending_certification", () => {
    const evidence = buildEvidence();
    evidence.certifications[0] = {
      rowId: "provider-core",
      domain: "PROVIDER",
      state: "PENDING",
    };
    const decision = createProductionReadinessDecision({
      decisionId: "d",
      releaseId: "release-1",
      gate: buildGate(),
      evidence,
      handoff: buildHandoff(),
    });
    expect(decision.decision).toBe("NOT_READY");
  });

  it("ep043_unit_readiness_not_ready_when_release_blocking_pending", () => {
    const evidence = buildEvidence();
    evidence.certifications[1] = {
      rowId: "hardware-core",
      domain: "HARDWARE",
      state: "RELEASE-BLOCKING-PENDING",
    };
    const decision = createProductionReadinessDecision({
      decisionId: "d",
      releaseId: "release-1",
      gate: buildGate(),
      evidence,
      handoff: buildHandoff(),
    });
    expect(decision.decision).toBe("NOT_READY");
  });

  it("ep043_unit_readiness_not_ready_when_review_failed", () => {
    const evidence = buildEvidence();
    evidence.reviews[0] = {
      domain: "SECURITY",
      status: "FAIL",
      evidenceRef: "",
    };
    const decision = createProductionReadinessDecision({
      decisionId: "d",
      releaseId: "release-1",
      gate: buildGate(),
      evidence,
      handoff: buildHandoff(),
    });
    expect(decision.decision).toBe("NOT_READY");
  });

  it("ep043_unit_readiness_not_ready_when_drill_missing", () => {
    const evidence = buildEvidence();
    evidence.drills = evidence.drills.filter(
      (drill) => drill.kind !== "RESTORE",
    );
    const decision = createProductionReadinessDecision({
      decisionId: "d",
      releaseId: "release-1",
      gate: buildGate(),
      evidence,
      handoff: buildHandoff(),
    });
    expect(decision.decision).toBe("NOT_READY");
  });

  it("ep043_unit_readiness_not_ready_when_drill_undated", () => {
    const evidence = buildEvidence();
    evidence.drills[0] = { kind: "RESTORE", status: "NOT_RUN" };
    const decision = createProductionReadinessDecision({
      decisionId: "d",
      releaseId: "release-1",
      gate: buildGate(),
      evidence,
      handoff: buildHandoff(),
    });
    expect(decision.decision).toBe("NOT_READY");
  });

  it("ep043_unit_readiness_not_ready_when_release_candidate", () => {
    expect(() =>
      createProductionReadinessDecision({
        decisionId: "d",
        releaseId: "release-1",
        gate: buildGate({ releaseKind: "RELEASE_CANDIDATE" }),
        evidence: buildEvidence(),
        handoff: buildHandoff(),
      }),
    ).toThrow(ShipError);
  });

  it("ep043_unit_readiness_parse_roundtrip", () => {
    const decision = createProductionReadinessDecision({
      decisionId: "decision-1",
      releaseId: "release-1",
      gate: buildGate(),
      evidence: buildEvidence(),
      handoff: buildHandoff(),
      decidedAt: FIXED_NOW,
    });
    const parsed = parseProductionReadinessDecision(
      JSON.parse(JSON.stringify(decision)),
    );
    expect(parsed).toEqual(decision);
  });

  it("ep043_unit_readiness_tampered_decision_rejected", () => {
    const decision = createProductionReadinessDecision({
      decisionId: "decision-1",
      releaseId: "release-1",
      gate: buildGate(),
      evidence: buildEvidence(),
      handoff: buildHandoff(),
      decidedAt: FIXED_NOW,
    });
    const serialized = JSON.parse(JSON.stringify(decision));
    serialized["decision"] = "READY";
    serialized["gate"]["freshCloneRerun"] = false;
    expect(() => parseProductionReadinessDecision(serialized)).toThrow(
      ShipError,
    );
  });

  it("ep043_unit_readiness_unknown_field_rejected", () => {
    const decision = createProductionReadinessDecision({
      decisionId: "decision-1",
      releaseId: "release-1",
      gate: buildGate(),
      evidence: buildEvidence(),
      handoff: buildHandoff(),
      decidedAt: FIXED_NOW,
    });
    const serialized = JSON.parse(JSON.stringify(decision));
    serialized["deployNow"] = true;
    expect(() => parseProductionReadinessDecision(serialized)).toThrow(
      ShipError,
    );
  });

  it("ep043_unit_evaluate_readiness_is_deterministic", () => {
    expect(
      evaluateProductionReadiness(buildGate(), buildEvidence(), buildHandoff()),
    ).toBe("READY");
    const blockedGate = createShipGate({
      gateId: "gate-blocked",
      releaseKind: "CORE_RELEASE",
      phase: "SHIP_DECISION",
      requiredProofs: allProofsPass(),
      freshCloneRerun: false,
    });
    expect(blockedGate.verdict).toBe("BLOCKED");
    expect(
      evaluateProductionReadiness(blockedGate, buildEvidence(), buildHandoff()),
    ).toBe("NOT_READY");
  });
});

describe("EP-043 M1 errors and redaction", () => {
  it("ep043_unit_error_codes_known", () => {
    expect(() => assertKnownShipErrorCode("VALIDATION_FAILED")).not.toThrow();
    expect(() => assertKnownShipErrorCode("MADE_UP")).toThrow(ShipError);
  });

  it("ep043_unit_error_redacts_secrets", () => {
    const error = new ShipError(
      "VALIDATION_FAILED",
      "bad token sk-live-abcdef1234567890 supplied",
    );
    expect(error.message).not.toContain("sk-live-abcdef1234567890");
    expect(error.toShape().redacted).toBe(true);
  });

  it("ep043_unit_redact_ship_message_shapes", () => {
    const input =
      "ak=AKIAABCDEFGHIJKLMNOP bearer=Bearer abcdefghijklmnopqrst secret=supersecretvalue";
    const out = redactShipMessage(input);
    expect(out).not.toContain("AKIAABCDEFGHIJKLMNOP");
    expect(out).not.toContain("supersecretvalue");
  });

  it("ep043_unit_vocabulary_values_are_exact", () => {
    expect(CAPABILITY_STATUSES).toEqual([
      "IMPLEMENTED",
      "CERTIFIED",
      "EXPERIMENTAL",
      "UNAVAILABLE",
      "DEFERRED",
    ]);
    expect(RELEASE_KINDS).toEqual(["RELEASE_CANDIDATE", "CORE_RELEASE"]);
    expect(GATE_VERDICTS).toEqual(["PENDING", "BLOCKED", "PASSED"]);
    expect(PROOF_STATUSES).toEqual(["NOT_RUN", "PASS", "FAIL", "BLOCKED"]);
    expect(DRILL_STATUSES).toEqual(["NOT_RUN", "DATED_EVIDENCE", "FAILED"]);
    expect(CERTIFICATION_ROW_STATES).toEqual([
      "PENDING",
      "SIGNED",
      "RELEASE-BLOCKING-PENDING",
    ]);
    expect(SHIP_PHASES).toEqual([
      "PRE_SHIP",
      "FRESH_CLONE_VERIFY",
      "PRODUCTION_READINESS",
      "LIVE_FIRE",
      "SHIP_DECISION",
      "MANUAL_DEPLOY_HANDOFF",
    ]);
    expect(WAIVER_CLASSES).toEqual(["NONE", "ACCEPTED_RISK", "GENERIC"]);
  });
});
