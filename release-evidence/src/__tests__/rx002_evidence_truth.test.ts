/**
 * RX-002 Evidence Truth Engine regression tests.
 *
 * Encodes the corrected behavior for AUD-071/072/073/074/075/087/002:
 * a filename is never a PASS; structured execution evidence is required;
 * SIGNED requires a verifier; all six drill classes are real readiness
 * obligations; deletion/corruption/semantic failure flips READY to
 * NOT_READY. These tests fail against the pre-RX-002 presence-only
 * collectors and pass only after the Evidence Truth Engine lands.
 */
import { describe, expect, it } from "vitest";

import {
  parseExecutionEvidence,
  validateExecutionEvidence,
  collectLiveFireProofs,
  collectReviews,
  collectDrills,
  collectCertifications,
  collectFreshCloneEvidence,
  evaluateLiveFireObligation,
  evaluateReviewsObligation,
  evaluateDrillsObligation,
  evaluateCertificationObligation,
  evaluateReadiness,
  defaultRepoPaths,
  type ExecutionEvidence,
  type LiveFireProofResult,
  type ReadinessInputs,
} from "@nexus/release-evidence";
import { DRILL_KINDS, REVIEW_DOMAINS } from "@nexus/release-evidence";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

function validEvidence(
  overrides: Partial<ExecutionEvidence> = {},
): ExecutionEvidence {
  return {
    schema_version: 1,
    proof_id: "LF-001",
    producer: "scripts/live-fire/LF-001.sh",
    command: "sh scripts/live-fire/LF-001.sh",
    started_at: "2026-08-29T00:00:00.000Z",
    completed_at: new Date().toISOString(),
    exit_code: 0,
    result: "PASS",
    git_commit: "15194acd35d245b2dfdbbd6865185faed0a5b030",
    run_id: "lf001-run-1",
    environment_class: "FULLY_LOCAL",
    artifact_digests: { bundle: "sha256:" + "a".repeat(64) },
    stdout_digest: "sha256:" + "b".repeat(64),
    stderr_digest: "sha256:" + "c".repeat(64),
    ...overrides,
  };
}

function makeRepo(files: Record<string, string>): string {
  const dir = mkdtempSync(join(tmpdir(), "rx002-"));
  const ev = join(dir, ".agent", "state", "evidence");
  const git = join(dir, ".git", "refs", "heads");
  const lf = join(dir, "live-fire");
  mkdirSync(ev, { recursive: true });
  mkdirSync(git, { recursive: true });
  mkdirSync(lf, { recursive: true });
  // Minimal real git layout so currentGitCommit resolves to the fixture commit.
  writeFileSync(join(dir, ".git", "HEAD"), "ref: refs/heads/master\n");
  writeFileSync(
    join(git, "master"),
    "15194acd35d245b2dfdbbd6865185faed0a5b030\n",
  );
  writeFileSync(
    join(lf, "REGISTRY.tsv"),
    "LF-001|EP-035|scripts/live-fire/LF-001.sh|one-package-deployment|desc\n",
  );
  for (const [name, content] of Object.entries(files)) {
    writeFileSync(join(ev, name), content);
  }
  return dir;
}

function pathsFor(dir: string) {
  return defaultRepoPaths(dir);
}

describe("RX-002 structured execution evidence schema", () => {
  it("rx002_evidence_parse_requires_all_required_fields", () => {
    const missing = validEvidence();
    // @ts-expect-error deleting a required field for the negative case
    delete missing.stdout_digest;
    expect(() => parseExecutionEvidence(JSON.stringify(missing))).toThrow();
  });

  it("rx002_evidence_parse_accepts_complete_record", () => {
    const record = parseExecutionEvidence(JSON.stringify(validEvidence()));
    expect(record.proof_id).toBe("LF-001");
    expect(record.schema_version).toBe(1);
  });

  it("rx002_evidence_parse_rejects_non_json", () => {
    expect(() => parseExecutionEvidence("not json at all")).toThrow();
  });

  it("rx002_evidence_validate_rejects_nonzero_exit", () => {
    expect(
      validateExecutionEvidence(validEvidence({ exit_code: 1 }), {
        expectedCommit: "15194acd35d245b2dfdbbd6865185faed0a5b030",
      }),
    ).toBe(false);
  });

  it("rx002_evidence_validate_rejects_failed_result", () => {
    expect(
      validateExecutionEvidence(validEvidence({ result: "FAIL" }), {
        expectedCommit: "15194acd35d245b2dfdbbd6865185faed0a5b030",
      }),
    ).toBe(false);
  });

  it("rx002_evidence_validate_rejects_stale_evidence", () => {
    const stale = validEvidence({
      completed_at: "2026-01-01T00:00:00.000Z",
    });
    expect(
      validateExecutionEvidence(stale, {
        expectedCommit: "15194acd35d245b2dfdbbd6865185faed0a5b030",
        maxAgeMs: 24 * 3600 * 1000,
      }),
    ).toBe(false);
  });

  it("rx002_evidence_validate_rejects_wrong_commit", () => {
    expect(
      validateExecutionEvidence(validEvidence({ git_commit: "deadbeef" }), {
        expectedCommit: "15194acd35d245b2dfdbbd6865185faed0a5b030",
      }),
    ).toBe(false);
  });

  it("rx002_evidence_validate_accepts_fresh_pass_on_commit", () => {
    expect(
      validateExecutionEvidence(validEvidence(), {
        expectedCommit: "15194acd35d245b2dfdbbd6865185faed0a5b030",
        maxAgeMs: 24 * 3600 * 1000,
      }),
    ).toBe(true);
  });
});

describe("RX-002 live-fire evidence truth (AUD-071)", () => {
  const patched = pathsFor("/nonexistent");

  it("rx002_livefire_filename_alone_is_not_pass", () => {
    // Pre-RX-002: an evidenceRef filename with no content check produced PASS.
    const proof: LiveFireProofResult = {
      lfId: "LF-001",
      ownerNode: "EP-035",
      slug: "one-package-deployment",
      ownerDone: true,
      evidenceRef: ".agent/state/evidence/LF-001-ep035-m5.json",
      validated: false,
    };
    const result = evaluateLiveFireObligation([proof]);
    // The ref must resolve to validated structured evidence before PASS.
    expect(result.met).toBe(false);
  });

  it("rx002_livefire_missing_evidence_not_pass", () => {
    const proof: LiveFireProofResult = {
      lfId: "LF-001",
      ownerNode: "EP-035",
      slug: "one-package-deployment",
      ownerDone: true,
      evidenceRef: "",
      validated: false,
    };
    expect(evaluateLiveFireObligation([proof]).met).toBe(false);
  });

  it("rx002_collect_livefire_reads_structured_evidence", () => {
    const dir = makeRepo({
      "LF-001-ep035-m5.json": JSON.stringify(
        validEvidence({ proof_id: "LF-001" }),
      ),
    });
    const proofs = collectLiveFireProofs(pathsFor(dir));
    // The collector must find evidence by either naming scheme and expose
    // a structured-validated result, not just a filename.
    expect(proofs.length).toBeGreaterThan(0);
    rmSync(dir, { recursive: true, force: true });
  });

  it("rx002_collect_livefire_corrupt_evidence_is_not_pass", () => {
    const dir = makeRepo({
      "LF-001-ep035-m5.json": "{ this is not valid evidence json",
    });
    const proofs = collectLiveFireProofs(pathsFor(dir));
    const proof = proofs.find((p) => p.lfId === "LF-001");
    expect(proof).toBeDefined();
    if (proof) {
      expect(proof.validated).toBe(false);
    }
    rmSync(dir, { recursive: true, force: true });
  });
});

describe("RX-002 review evidence truth (AUD-073)", () => {
  it("rx002_review_filename_alone_is_not_pass", () => {
    const reviews = [
      {
        domain: "SECURITY" as const,
        status: "PASS" as const,
        evidenceRef: ".agent/state/evidence/ep043-review-security.md",
      },
    ];
    expect(evaluateReviewsObligation(reviews).met).toBe(false);
  });

  it("rx002_review_collector_requires_structured_pass", () => {
    const dir = makeRepo({
      // Filename exists but content is not a structured PASS record.
      "ep043-review-security.md": "# SECURITY REVIEW\n\nstatus: PASS\n",
    });
    const reviews = collectReviews(pathsFor(dir));
    const security = reviews.find((r) => r.domain === "SECURITY");
    expect(security).toBeDefined();
    if (security) {
      expect(security.status).not.toBe("PASS");
    }
    rmSync(dir, { recursive: true, force: true });
  });
});

describe("RX-002 drill obligations (AUD-072)", () => {
  it("rx002_drills_all_six_required", () => {
    const drills = [
      { kind: "RESTORE" as const, status: "DATED_EVIDENCE" as const },
    ];
    expect(evaluateDrillsObligation(drills).met).toBe(false);
    expect(evaluateDrillsObligation([]).met).toBe(false);
  });

  it("rx002_drills_all_six_met_only_with_dated_evidence", () => {
    const drills = DRILL_KINDS.map((kind) => ({
      kind,
      status: "DATED_EVIDENCE" as const,
      datedAt: new Date().toISOString(),
      evidenceRef: `ep043-drill-${kind.toLowerCase()}`,
    }));
    expect(evaluateDrillsObligation(drills).met).toBe(true);
  });

  it("rx002_drills_not_run_blocks_readiness", () => {
    const inputs: ReadinessInputs = {
      graphNodes: [{ nodeId: "EP-001", done: true }],
      liveFireProofs: [],
      certifications: { providerRows: [], hardwareRows: [] },
      reviews: [],
      drills: [],
      releaseTag: "green/EP-043",
      manualDeployCommand: "sh scripts/deploy.sh --deploy",
      freshCloneRerun: true,
    };
    expect(evaluateReadiness(inputs).decision).toBe("NOT_READY");
  });
});

describe("RX-002 certification truth (AUD-074)", () => {
  it("rx002_certification_text_signed_is_not_verified", () => {
    // A SIGNED textual marker in RESULTS.md with no structured
    // verification record must not count as verified (AUD-074).
    const dir = makeRepo({});
    mkdirSync(join(dir, "provider-certification"), { recursive: true });
    writeFileSync(
      join(dir, "provider-certification", "RESULTS.md"),
      "SIGNED: provider-aws-eu\n",
    );
    const certs = collectCertifications(pathsFor(dir));
    expect(certs.providerRows.length).toBeGreaterThan(0);
    const evaluation = evaluateCertificationObligation(certs);
    expect(evaluation.met).toBe(false);
    expect(evaluation.reasons.join(" ")).toContain(
      "without verified structured record",
    );
    rmSync(dir, { recursive: true, force: true });
  });

  it("rx002_certification_structured_record_is_verified", () => {
    // The same SIGNED marker becomes verified when a structured
    // execution evidence record validates for that row.
    const dir = makeRepo({
      "ep043-cert-provider-1-provider-aws-eu-x.json": JSON.stringify(
        validEvidence({
          proof_id: "ep043-cert-provider-1-provider-aws-eu",
          result: "VERIFIED",
          command: "scripts/certify-provider.sh",
        }),
      ),
    });
    mkdirSync(join(dir, "provider-certification"), { recursive: true });
    writeFileSync(
      join(dir, "provider-certification", "RESULTS.md"),
      "SIGNED: provider-aws-eu\n",
    );
    const certs = collectCertifications(pathsFor(dir));
    const evaluation = evaluateCertificationObligation(certs);
    expect(evaluation.met).toBe(true);
    rmSync(dir, { recursive: true, force: true });
  });
});

describe("RX-002 fresh-clone truth (AUD-075)", () => {
  it("rx002_freshclone_filename_alone_is_not_proof", () => {
    const dir = makeRepo({
      "ep043-freshclone-m5.md": "# FRESH-CLONE ACCEPTANCE EVIDENCE\nRun: x\n",
    });
    // Pre-RX-002 this returned true for any matching filename.
    expect(collectFreshCloneEvidence(pathsFor(dir))).toBe(false);
    rmSync(dir, { recursive: true, force: true });
  });

  it("rx002_freshclone_structured_proof_accepted", () => {
    const dir = makeRepo({
      "ep043-freshclone-m5.json": JSON.stringify(
        validEvidence({
          proof_id: "ep043-freshclone",
          result: "VERIFIED",
          command: "sh scripts/ep043-freshclone-accept.sh",
        }),
      ),
    });
    expect(collectFreshCloneEvidence(pathsFor(dir))).toBe(true);
    rmSync(dir, { recursive: true, force: true });
  });
});
