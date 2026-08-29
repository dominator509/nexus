/**
 * EP-043 M2 production readiness evaluation (SPEC-008).
 *
 * Pure domain: deterministic evaluation of the five acceptance
 * obligations from the node contract:
 *   1. All graph nodes are DONE
 *   2. All live-fire proofs pass
 *   3. Required provider and hardware certification rows are signed
 *   4. Mandatory reviews pass (security, privacy, performance,
 *      accessibility, observability, backup, restore, update, rollback)
 *   5. A release tag and exact manual deploy command exist
 *
 * I/O lives in repo-state.ts (adapters); this module is pure and
 * fail-closed. A green evaluation requires every obligation to be met
 * with real evidence references; any missing/pending item blocks.
 *
 * Honest verdicts: the engine reports NOT_READY with exact blocking
 * reasons when obligations are unmet. It never fabricates readiness.
 */

import {
  DRILL_KINDS,
  REVIEW_DOMAINS,
  type CertificationRow,
  type DrillEvidence,
  type ProofStatus,
  type ReviewResult,
  type ShipGateBlock,
  type ShipGateProof,
  type WaiverClass,
} from "./model.ts";
import { ShipError } from "./errors.ts";

/** Live-fire proof result as collected from the registry + ledger. */
export interface LiveFireProofResult {
  lfId: string;
  ownerNode: string;
  slug: string;
  /** true when the owning node is DONE (proof may run). */
  ownerDone: boolean;
  /** evidence file relative path; empty when missing. */
  evidenceRef: string;
  /** true only when the evidence file is a validated structured record. */
  validated: boolean;
}

/** Graph node status as collected from GRAPH.md + LEDGER. */
export interface GraphNodeStatus {
  nodeId: string;
  done: boolean;
}

/** Certification matrix rows as collected from RESULTS.md files. */
export interface CertificationMatrixInput {
  providerRows: CertificationRow[];
  hardwareRows: CertificationRow[];
}

/** Reviews collected from gate evidence. */
export interface ReviewInput {
  domain: (typeof REVIEW_DOMAINS)[number];
  status: ProofStatus;
  evidenceRef: string;
}

/** Drills collected from evidence dir. */
export interface DrillInput {
  kind: (typeof DRILL_KINDS)[number];
  status: "NOT_RUN" | "DATED_EVIDENCE" | "FAILED";
  datedAt?: string;
  evidenceRef?: string;
}

/** All typed inputs for one readiness evaluation. */
export interface ReadinessInputs {
  graphNodes: GraphNodeStatus[];
  liveFireProofs: LiveFireProofResult[];
  certifications: CertificationMatrixInput;
  reviews: ReviewInput[];
  drills: DrillInput[];
  releaseTag: string;
  manualDeployCommand: string;
  freshCloneRerun: boolean;
}

/** Result of evaluating one acceptance obligation. */
export interface ObligationResult {
  obligation: string;
  met: boolean;
  reasons: string[];
}

/** Full readiness evaluation result. */
export interface ReadinessEvaluation {
  obligations: ObligationResult[];
  allMet: boolean;
  blockingReasons: string[];
  shipGateVerdict: "PENDING" | "BLOCKED" | "PASSED";
  decision: "READY" | "NOT_READY";
}

function block(
  code: string,
  message: string,
  waiverClass: WaiverClass = "NONE",
): ShipGateBlock {
  return { code, message, waiverClass };
}

/** Build ShipGateProof[] from live-fire results. */
export function liveFireProofsToGateProofs(
  results: LiveFireProofResult[],
): ShipGateProof[] {
  return results.map((result) => ({
    family: "LIVE_FIRE",
    proofId: result.lfId,
    status:
      result.ownerDone && result.validated && result.evidenceRef.length > 0
        ? "PASS"
        : "NOT_RUN",
    evidenceRef: result.evidenceRef,
  }));
}

/** Obligation 1: all graph nodes DONE. */
export function evaluateGraphObligation(
  nodes: GraphNodeStatus[],
): ObligationResult {
  const reasons: string[] = [];
  for (const node of nodes) {
    if (!node.done) {
      reasons.push(`graph node ${node.nodeId} is not DONE`);
    }
  }
  return {
    obligation: "all graph nodes are DONE",
    met: reasons.length === 0,
    reasons,
  };
}

/** Obligation 2: all live-fire proofs pass with validated evidence. */
export function evaluateLiveFireObligation(
  proofs: LiveFireProofResult[],
): ObligationResult {
  const reasons: string[] = [];
  if (proofs.length === 0) {
    reasons.push("no live-fire proofs registered (vacuity guard)");
  }
  for (const proof of proofs) {
    if (!proof.ownerDone) {
      reasons.push(`${proof.lfId} owner ${proof.ownerNode} is not DONE`);
    } else if (proof.evidenceRef.length === 0) {
      reasons.push(`${proof.lfId} has no evidence file`);
    } else if (!proof.validated) {
      reasons.push(
        `${proof.lfId} evidence is not a validated structured record (exit 0, PASS result, current commit, fresh)`,
      );
    }
  }
  return {
    obligation: "all live-fire proofs pass",
    met: reasons.length === 0,
    reasons,
  };
}

/** Obligation 3: all six required drill classes have dated, validated evidence. */
export function evaluateDrillsObligation(
  drills: DrillInput[],
): ObligationResult {
  const reasons: string[] = [];
  for (const kind of DRILL_KINDS) {
    const drill = drills.find((item) => item.kind === kind);
    if (!drill) {
      reasons.push(`drill ${kind} is missing`);
    } else if (drill.status !== "DATED_EVIDENCE") {
      reasons.push(`drill ${kind} has no dated evidence (${drill.status})`);
    } else if (!drill.evidenceRef || drill.evidenceRef.length === 0) {
      reasons.push(`drill ${kind} dated evidence has no evidence ref`);
    }
  }
  return {
    obligation:
      "restore, rollback, provider-failover, identity-recovery, sentinel-containment, and update-failure drills pass with dated evidence",
    met: reasons.length === 0,
    reasons,
  };
}

/** Obligation 4: required certification rows signed. */
export function evaluateCertificationObligation(
  input: CertificationMatrixInput,
): ObligationResult {
  const reasons: string[] = [];
  const rows = [...input.providerRows, ...input.hardwareRows];
  if (rows.length === 0) {
    reasons.push("no certification rows present");
  }
  for (const row of rows) {
    if (row.state === "RELEASE-BLOCKING-PENDING") {
      reasons.push(
        `certification row ${row.rowId} is RELEASE-BLOCKING-PENDING`,
      );
    } else if (row.state === "PENDING") {
      reasons.push(`certification row ${row.rowId} is PENDING`);
    } else if (
      row.state === "SIGNED" &&
      (!row.evidenceRef || row.evidenceRef.length === 0)
    ) {
      reasons.push(
        `certification row ${row.rowId} signed without evidence ref`,
      );
    }
  }
  return {
    obligation: "required provider and hardware certification rows are signed",
    met: reasons.length === 0,
    reasons,
  };
}

/** Obligation 4: mandatory reviews pass. */
export function evaluateReviewsObligation(
  reviews: ReviewInput[],
): ObligationResult {
  const reasons: string[] = [];
  const domains = new Set(reviews.map((review) => review.domain));
  for (const domain of REVIEW_DOMAINS) {
    if (!domains.has(domain)) {
      reasons.push(`review ${domain} is missing`);
      continue;
    }
    const review = reviews.find((item) => item.domain === domain);
    if (!review || review.status !== "PASS") {
      reasons.push(`review ${domain} is not PASS`);
    } else if (review.evidenceRef.length === 0) {
      reasons.push(`review ${domain} passed without evidence ref`);
    }
  }
  return {
    obligation:
      "security, privacy, performance, accessibility, observability, backup, restore, update, and rollback reviews pass",
    met: reasons.length === 0,
    reasons,
  };
}

/** Obligation 5: release tag + exact manual deploy command exist. */
export function evaluateReleaseObligation(
  releaseTag: string,
  manualDeployCommand: string,
): ObligationResult {
  const reasons: string[] = [];
  if (releaseTag.length === 0) {
    reasons.push("no release tag");
  }
  if (manualDeployCommand.length === 0) {
    reasons.push("no exact manual deploy command");
  }
  return {
    obligation:
      "a release tag and exact manual deploy command are produced without deploying production",
    met: reasons.length === 0,
    reasons,
  };
}

/** Deterministic full readiness evaluation. Never trusts input verdicts. */
export function evaluateReadiness(
  inputs: ReadinessInputs,
): ReadinessEvaluation {
  const obligations: ObligationResult[] = [
    evaluateGraphObligation(inputs.graphNodes),
    evaluateLiveFireObligation(inputs.liveFireProofs),
    evaluateDrillsObligation(inputs.drills),
    evaluateCertificationObligation(inputs.certifications),
    evaluateReviewsObligation(inputs.reviews),
    evaluateReleaseObligation(inputs.releaseTag, inputs.manualDeployCommand),
  ];

  const blockingReasons: string[] = [];
  for (const obligation of obligations) {
    if (!obligation.met) {
      blockingReasons.push(...obligation.reasons);
    }
  }
  if (!inputs.freshCloneRerun) {
    blockingReasons.push("fresh-clone-equivalent rerun has not been executed");
  }

  const allMet = blockingReasons.length === 0;
  const shipGateVerdict: "PENDING" | "BLOCKED" | "PASSED" = allMet
    ? "PASSED"
    : "BLOCKED";
  const decision: "READY" | "NOT_READY" = allMet ? "READY" : "NOT_READY";

  return { obligations, allMet, blockingReasons, shipGateVerdict, decision };
}

/** Build ShipGateBlock[] from evaluation blocking reasons. */
export function evaluationBlocks(
  evaluation: ReadinessEvaluation,
): ShipGateBlock[] {
  return evaluation.blockingReasons.map((reason) => block("READINESS", reason));
}

/** Validate the inputs are well-formed (deny unknown review/drill kinds). */
export function validateReadinessInputs(inputs: ReadinessInputs): void {
  for (const review of inputs.reviews) {
    if (!(REVIEW_DOMAINS as readonly string[]).includes(review.domain)) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown review domain: ${String(review.domain)}`,
      );
    }
  }
  for (const drill of inputs.drills) {
    if (!(DRILL_KINDS as readonly string[]).includes(drill.kind)) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown drill kind: ${String(drill.kind)}`,
      );
    }
    if (
      drill.status === "DATED_EVIDENCE" &&
      (!drill.datedAt || drill.datedAt.length === 0)
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `dated drill requires timestamp: ${drill.kind}`,
      );
    }
  }
}

/** Build the ReleaseEvidence certifications list from matrix input. */
export function certificationRowsToEvidence(
  input: CertificationMatrixInput,
): CertificationRow[] {
  return [...input.providerRows, ...input.hardwareRows];
}

/** Build DrillEvidence[] from drill input. */
export function drillsToEvidence(drills: DrillInput[]): DrillEvidence[] {
  return drills.map((drill) => ({
    kind: drill.kind,
    status: drill.status,
    ...(drill.datedAt ? { datedAt: drill.datedAt } : {}),
    ...(drill.evidenceRef ? { evidenceRef: drill.evidenceRef } : {}),
  }));
}

/** Build ReviewResult[] from review input. */
export function reviewsToEvidence(reviews: ReviewInput[]): ReviewResult[] {
  return reviews.map((review) => ({
    domain: review.domain,
    status: review.status,
    evidenceRef: review.evidenceRef,
  }));
}
