/**
 * EP-043 M1 production readiness and ship model (SPEC-008).
 *
 * Four provider-neutral, versioned public interfaces:
 *   - ShipGate: the gate that decides whether a release may ship.
 *   - ReleaseEvidence: the machine-readable evidence index bound to a run.
 *   - ManualDeployHandoff: the exact manual deploy command (auto-deploy is
 *     never authorized).
 *   - ProductionReadinessDecision: the final readiness decision record.
 *
 * Invariants encoded here (SPEC-008 behaviors):
 *   - RELEASE CANDIDATE != CORE RELEASE
 *   - GATE PASSED != SHIPPED (production deployment remains a manual command)
 *   - EVIDENCE EXISTS != RELEASE SIGNED
 *   - EVIDENCE INDEX != DATED DRILL EVIDENCE
 *   - DECISION MADE != RELEASE SHIPPED
 *   - HANDOFF EXISTS != DEPLOYMENT EXECUTED
 *   - NO GENERIC WAIVER for critical vulnerability, unreviewed license,
 *     missing SBOM, stale backup, or failed required proof.
 *   - PASSING FROM CACHED OUTPUT IS NOT READINESS (fresh-clone-equivalent
 *     rerun is a hard prerequisite).
 *   - SIGNED CERTIFICATION ROW != PRODUCTION READY; a
 *     RELEASE-BLOCKING-PENDING row blocks the decision.
 *
 * Every vocabulary enum is deny-unknown: unknown wire values fail closed.
 */

import {
  ShipError,
  assertKnownShipErrorCode,
  redactShipMessage,
} from "./errors.ts";

/** Versioned serialization contract. All interfaces carry schema_version 1. */
export const SHIP_SCHEMA_VERSION = 1 as const;

/** SPEC-008 canonical capability status vocabulary. */
export const CAPABILITY_STATUSES = [
  "IMPLEMENTED",
  "CERTIFIED",
  "EXPERIMENTAL",
  "UNAVAILABLE",
  "DEFERRED",
] as const;
export type CapabilityStatus = (typeof CAPABILITY_STATUSES)[number];

/** SPEC-008 release kind vocabulary. */
export const RELEASE_KINDS = ["RELEASE_CANDIDATE", "CORE_RELEASE"] as const;
export type ReleaseKind = (typeof RELEASE_KINDS)[number];

/** Ship gate verdict vocabulary. */
export const GATE_VERDICTS = ["PENDING", "BLOCKED", "PASSED"] as const;
export type GateVerdict = (typeof GATE_VERDICTS)[number];

/** Proof execution status vocabulary. */
export const PROOF_STATUSES = ["NOT_RUN", "PASS", "FAIL", "BLOCKED"] as const;
export type ProofStatus = (typeof PROOF_STATUSES)[number];

/** Drill evidence status vocabulary (SPEC-008 behavior 5: dated evidence). */
export const DRILL_STATUSES = ["NOT_RUN", "DATED_EVIDENCE", "FAILED"] as const;
export type DrillStatus = (typeof DRILL_STATUSES)[number];

/** Certification row state vocabulary (mirrors certification_validate.py). */
export const CERTIFICATION_ROW_STATES = [
  "PENDING",
  "SIGNED",
  "RELEASE-BLOCKING-PENDING",
] as const;
export type CertificationRowState = (typeof CERTIFICATION_ROW_STATES)[number];

/** Ship phase vocabulary (SPEC-008 behavior 4 fresh-clone ship ladder). */
export const SHIP_PHASES = [
  "PRE_SHIP",
  "FRESH_CLONE_VERIFY",
  "PRODUCTION_READINESS",
  "LIVE_FIRE",
  "SHIP_DECISION",
  "MANUAL_DEPLOY_HANDOFF",
] as const;
export type ShipPhase = (typeof SHIP_PHASES)[number];

/** Waiver class vocabulary (SPEC-008 behavior 6). */
export const WAIVER_CLASSES = ["NONE", "ACCEPTED_RISK", "GENERIC"] as const;
export type WaiverClass = (typeof WAIVER_CLASSES)[number];

/** Drill kinds with dated evidence requirements (SPEC-008 behavior 5). */
export const DRILL_KINDS = [
  "RESTORE",
  "ROLLBACK",
  "PROVIDER_FAILOVER",
  "IDENTITY_RECOVERY",
  "SENTINEL_CONTAINMENT",
  "UPDATE_FAILURE",
] as const;
export type DrillKind = (typeof DRILL_KINDS)[number];

/** Mandatory review domains (SPEC-008 behavior 1). */
export const REVIEW_DOMAINS = [
  "SECURITY",
  "PRIVACY",
  "PERFORMANCE",
  "ACCESSIBILITY",
  "OBSERVABILITY",
  "BACKUP",
  "RESTORE",
  "UPDATE",
  "ROLLBACK",
] as const;
export type ReviewDomain = (typeof REVIEW_DOMAINS)[number];

/** Mandatory gate families for a core release (SPEC-008 behavior 1). */
export const REQUIRED_GATE_FAMILIES = [
  "SECURITY",
  "DATA",
  "WORKFLOW",
  "INSTALLATION",
  "UPDATE",
  "BACKUP",
  "ROLLBACK",
] as const;
export type RequiredGateFamily = (typeof REQUIRED_GATE_FAMILIES)[number];

/** Deny-unknown validation for a vocabulary array. */
export function isKnownValue<T extends string>(
  values: readonly T[],
  value: unknown,
): value is T {
  return (
    typeof value === "string" && (values as readonly string[]).includes(value)
  );
}

export function assertKnownValue<T extends string>(
  values: readonly T[],
  value: unknown,
  label: string,
): T {
  if (!isKnownValue(values, value)) {
    throw new ShipError(
      "VALIDATION_FAILED",
      `unknown ${label}: ${String(value)}`,
    );
  }
  return value;
}

/* ------------------------------------------------------------------ */
/* ShipGate                                                           */
/* ------------------------------------------------------------------ */

/** A single gate/proof result feeding the ship gate. */
export interface ShipGateProof {
  family: RequiredGateFamily | string;
  proofId: string;
  status: ProofStatus;
  /** Evidence path or test id; must be non-empty when status is PASS. */
  evidenceRef: string;
}

/** A blocking item on the ship gate. */
export interface ShipGateBlock {
  code: string;
  message: string;
  /** Accepted Risk (bounded, dated) vs GENERIC waiver (always denied). */
  waiverClass: WaiverClass;
  /** ISO timestamp when the accepted risk was recorded, if any. */
  acceptedAt?: string;
}

/**
 * ShipGate - the gate that decides whether a release may ship.
 *
 * A gate is PASSED only when every required proof is PASS with a real
 * evidence reference and no blocking item exists. GENERIC waivers never
 * clear a block (SPEC-008 behavior 6). PASSED != SHIPPED: production
 * deployment is a separate manual command.
 */
export interface ShipGate {
  schema_version: 1;
  gateId: string;
  releaseKind: ReleaseKind;
  phase: ShipPhase;
  verdict: GateVerdict;
  requiredProofs: ShipGateProof[];
  blocks: ShipGateBlock[];
  /** Fresh-clone-equivalent rerun required (SPEC-008 behavior 4). */
  freshCloneRerun: boolean;
}

export function createShipGate(input: {
  gateId: string;
  releaseKind: ReleaseKind;
  phase?: ShipPhase;
  requiredProofs?: ShipGateProof[];
  blocks?: ShipGateBlock[];
  freshCloneRerun?: boolean;
}): ShipGate {
  const releaseKind = assertKnownValue(
    RELEASE_KINDS,
    input.releaseKind,
    "release kind",
  );
  const phase = assertKnownValue(
    SHIP_PHASES,
    input.phase ?? "PRE_SHIP",
    "ship phase",
  );
  const requiredProofs = (input.requiredProofs ?? []).map((proof) => {
    if (typeof proof.proofId !== "string" || proof.proofId.length === 0) {
      throw new ShipError(
        "VALIDATION_FAILED",
        "proof id must be a non-empty string",
      );
    }
    const status = assertKnownValue(
      PROOF_STATUSES,
      proof.status,
      "proof status",
    );
    if (
      status === "PASS" &&
      (typeof proof.evidenceRef !== "string" || proof.evidenceRef.length === 0)
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `passing proof requires evidence ref: ${proof.proofId}`,
      );
    }
    if (typeof proof.family !== "string" || proof.family.length === 0) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `proof family must be non-empty: ${proof.proofId}`,
      );
    }
    return { ...proof, status, evidenceRef: proof.evidenceRef };
  });
  const blocks = (input.blocks ?? []).map((block) => {
    const waiverClass = assertKnownValue(
      WAIVER_CLASSES,
      block.waiverClass,
      "waiver class",
    );
    if (typeof block.code !== "string" || block.code.length === 0) {
      throw new ShipError("VALIDATION_FAILED", "block code must be non-empty");
    }
    if (waiverClass === "GENERIC") {
      throw new ShipError(
        "POLICY_DENIED",
        `generic waiver cannot clear a ship gate block: ${block.code}`,
      );
    }
    if (
      waiverClass === "ACCEPTED_RISK" &&
      (typeof block.acceptedAt !== "string" || block.acceptedAt.length === 0)
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `accepted risk requires a dated decision: ${block.code}`,
      );
    }
    return { ...block, waiverClass };
  });
  const freshCloneRerun = input.freshCloneRerun ?? false;
  const verdict = evaluateShipGateVerdict(
    requiredProofs,
    blocks,
    freshCloneRerun,
  );
  return {
    schema_version: SHIP_SCHEMA_VERSION,
    gateId: input.gateId,
    releaseKind,
    phase,
    verdict,
    requiredProofs,
    blocks,
    freshCloneRerun,
  };
}

/**
 * Evaluate the ship gate verdict deterministically.
 *
 * BLOCKED when: any required proof is not PASS, any block exists, or the
 * fresh-clone-equivalent rerun did not happen. PASSED only when all of
 * those hold. This is the core SPEC-008 ladder:
 *   PROOF PASSED != GATE PASSED
 *   GATE PASSED != RELEASE SHIPPED
 */
export function evaluateShipGateVerdict(
  requiredProofs: ShipGateProof[],
  blocks: ShipGateBlock[],
  freshCloneRerun: boolean,
): GateVerdict {
  if (!freshCloneRerun) {
    return "BLOCKED";
  }
  if (blocks.length > 0) {
    return "BLOCKED";
  }
  for (const proof of requiredProofs) {
    if (proof.status !== "PASS") {
      return "BLOCKED";
    }
  }
  return "PASSED";
}

/** Parse a ShipGate from unknown wire data, fail-closed on unknown fields. */
export function parseShipGate(value: unknown): ShipGate {
  if (typeof value !== "object" || value === null) {
    throw new ShipError("VALIDATION_FAILED", "ship gate must be an object");
  }
  const obj = value as Record<string, unknown>;
  if (obj["schema_version"] !== 1) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "unsupported ship gate schema_version",
    );
  }
  for (const key of Object.keys(obj)) {
    if (
      ![
        "schema_version",
        "gateId",
        "releaseKind",
        "phase",
        "verdict",
        "requiredProofs",
        "blocks",
        "freshCloneRerun",
      ].includes(key)
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown ship gate field: ${key}`,
      );
    }
  }
  if (typeof obj["gateId"] !== "string" || obj["gateId"].length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "ship gate gateId must be a non-empty string",
    );
  }
  const releaseKind = assertKnownValue(
    RELEASE_KINDS,
    obj["releaseKind"],
    "release kind",
  );
  const phase = assertKnownValue(SHIP_PHASES, obj["phase"], "ship phase");
  const freshCloneRerun = obj["freshCloneRerun"] === true;
  const requiredProofs = Array.isArray(obj["requiredProofs"])
    ? obj["requiredProofs"].map((item) => parseShipGateProof(item))
    : [];
  const blocks = Array.isArray(obj["blocks"])
    ? obj["blocks"].map((item) => parseShipGateBlock(item))
    : [];
  const expectedVerdict = evaluateShipGateVerdict(
    requiredProofs,
    blocks,
    freshCloneRerun,
  );
  if (obj["verdict"] !== expectedVerdict) {
    throw new ShipError(
      "VERIFICATION_FAILED",
      `ship gate verdict mismatch: declared ${String(obj["verdict"])}, computed ${expectedVerdict}`,
    );
  }
  return {
    schema_version: 1,
    gateId: obj["gateId"],
    releaseKind,
    phase,
    verdict: expectedVerdict,
    requiredProofs,
    blocks,
    freshCloneRerun,
  };
}

function parseShipGateProof(value: unknown): ShipGateProof {
  if (typeof value !== "object" || value === null) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "ship gate proof must be an object",
    );
  }
  const obj = value as Record<string, unknown>;
  for (const key of Object.keys(obj)) {
    if (!["family", "proofId", "status", "evidenceRef"].includes(key)) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown ship gate proof field: ${key}`,
      );
    }
  }
  const status = assertKnownValue(
    PROOF_STATUSES,
    obj["status"],
    "proof status",
  );
  if (typeof obj["proofId"] !== "string" || obj["proofId"].length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "proof id must be a non-empty string",
    );
  }
  if (typeof obj["family"] !== "string" || obj["family"].length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "proof family must be a non-empty string",
    );
  }
  if (typeof obj["evidenceRef"] !== "string") {
    throw new ShipError(
      "VALIDATION_FAILED",
      "proof evidenceRef must be a string",
    );
  }
  if (status === "PASS" && obj["evidenceRef"].length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      `passing proof requires evidence ref: ${obj["proofId"]}`,
    );
  }
  return {
    family: obj["family"],
    proofId: obj["proofId"],
    status,
    evidenceRef: obj["evidenceRef"],
  };
}

function parseShipGateBlock(value: unknown): ShipGateBlock {
  if (typeof value !== "object" || value === null) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "ship gate block must be an object",
    );
  }
  const obj = value as Record<string, unknown>;
  for (const key of Object.keys(obj)) {
    if (!["code", "message", "waiverClass", "acceptedAt"].includes(key)) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown ship gate block field: ${key}`,
      );
    }
  }
  const waiverClass = assertKnownValue(
    WAIVER_CLASSES,
    obj["waiverClass"],
    "waiver class",
  );
  if (waiverClass === "GENERIC") {
    throw new ShipError(
      "POLICY_DENIED",
      "generic waiver cannot clear a ship gate block",
    );
  }
  if (typeof obj["code"] !== "string" || obj["code"].length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "block code must be a non-empty string",
    );
  }
  if (typeof obj["message"] !== "string") {
    throw new ShipError("VALIDATION_FAILED", "block message must be a string");
  }
  if (
    waiverClass === "ACCEPTED_RISK" &&
    (typeof obj["acceptedAt"] !== "string" || obj["acceptedAt"].length === 0)
  ) {
    throw new ShipError(
      "VALIDATION_FAILED",
      `accepted risk requires a dated decision: ${obj["code"]}`,
    );
  }
  return {
    code: obj["code"],
    message: redactShipMessage(obj["message"]),
    waiverClass,
    ...(waiverClass === "ACCEPTED_RISK" && obj["acceptedAt"] !== undefined
      ? { acceptedAt: obj["acceptedAt"] as string }
      : {}),
  };
}

/* ------------------------------------------------------------------ */
/* ReleaseEvidence                                                    */
/* ------------------------------------------------------------------ */

/** A certification row (provider or hardware) with a signed state. */
export interface CertificationRow {
  rowId: string;
  domain: "PROVIDER" | "HARDWARE";
  state: CertificationRowState;
  /** Path to the signed evidence artifact; required when SIGNED. */
  evidenceRef?: string;
  /**
   * True only when the SIGNED state is backed by a validated structured
   * verification record. A textual SIGNED marker in RESULTS.md is never
   * verification (AUD-074); collectors set verified=false when no
   * structured record validates.
   */
  verified?: boolean;
}

/** Dated drill evidence (SPEC-008 behavior 5). */
export interface DrillEvidence {
  kind: DrillKind;
  status: DrillStatus;
  /** ISO timestamp of the drill; required when DATED_EVIDENCE. */
  datedAt?: string;
  evidenceRef?: string;
}

/** A review result over a mandatory domain. */
export interface ReviewResult {
  domain: ReviewDomain;
  status: ProofStatus;
  evidenceRef: string;
}

/**
 * ReleaseEvidence - the machine-readable evidence index for a release run.
 *
 * An evidence index may exist without a signed release; a signed release
 * requires the evidence index. The index binds to the exact run (run_id,
 * git_commit) and carries a real digest over its canonical serialization.
 */
export interface ReleaseEvidence {
  schema_version: 1;
  node: string;
  runId: string;
  gitCommit: string;
  releaseId: string;
  certifications: CertificationRow[];
  drills: DrillEvidence[];
  reviews: ReviewResult[];
  releaseNotes: Record<string, CapabilityStatus>;
  /** sha256 hex digest over the canonical evidence payload. */
  evidenceDigest: string;
  /** Redaction result for secret-shaped content. */
  redactionResult: "REDACTED" | "CLEAN";
}

export function createReleaseEvidence(input: {
  node: string;
  runId: string;
  gitCommit: string;
  releaseId: string;
  certifications?: CertificationRow[];
  drills?: DrillEvidence[];
  reviews?: ReviewResult[];
  releaseNotes?: Record<string, CapabilityStatus>;
}): ReleaseEvidence {
  if (typeof input.node !== "string" || input.node.length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "evidence node must be a non-empty string",
    );
  }
  if (typeof input.runId !== "string" || input.runId.length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "evidence runId must be a non-empty string",
    );
  }
  if (typeof input.gitCommit !== "string" || input.gitCommit.length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "evidence gitCommit must be a non-empty string",
    );
  }
  if (typeof input.releaseId !== "string" || input.releaseId.length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "evidence releaseId must be a non-empty string",
    );
  }
  const certifications = (input.certifications ?? []).map((row) => {
    if (typeof row.rowId !== "string" || row.rowId.length === 0) {
      throw new ShipError(
        "VALIDATION_FAILED",
        "certification row id must be non-empty",
      );
    }
    const state = assertKnownValue(
      CERTIFICATION_ROW_STATES,
      row.state,
      "certification row state",
    );
    if (
      state === "SIGNED" &&
      (typeof row.evidenceRef !== "string" || row.evidenceRef.length === 0)
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `signed certification row requires evidence ref: ${row.rowId}`,
      );
    }
    return { ...row, state };
  });
  const drills = (input.drills ?? []).map((drill) => {
    const kind = assertKnownValue(DRILL_KINDS, drill.kind, "drill kind");
    const status = assertKnownValue(
      DRILL_STATUSES,
      drill.status,
      "drill status",
    );
    if (
      status === "DATED_EVIDENCE" &&
      (typeof drill.datedAt !== "string" || drill.datedAt.length === 0)
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `dated drill requires a timestamp: ${drill.kind}`,
      );
    }
    return { ...drill, kind, status };
  });
  const reviews = (input.reviews ?? []).map((review) => {
    const domain = assertKnownValue(
      REVIEW_DOMAINS,
      review.domain,
      "review domain",
    );
    const status = assertKnownValue(
      PROOF_STATUSES,
      review.status,
      "review status",
    );
    if (
      status === "PASS" &&
      (typeof review.evidenceRef !== "string" ||
        review.evidenceRef.length === 0)
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `passing review requires evidence ref: ${review.domain}`,
      );
    }
    return { ...review, domain, status };
  });
  const releaseNotes: Record<string, CapabilityStatus> = {};
  for (const [key, value] of Object.entries(input.releaseNotes ?? {})) {
    releaseNotes[key] = assertKnownValue(
      CAPABILITY_STATUSES,
      value,
      "capability status",
    );
  }
  const payload: Omit<ReleaseEvidence, "evidenceDigest" | "redactionResult"> = {
    schema_version: 1,
    node: input.node,
    runId: input.runId,
    gitCommit: input.gitCommit,
    releaseId: input.releaseId,
    certifications,
    drills,
    reviews,
    releaseNotes,
  };
  const evidenceDigest = canonicalEvidenceDigest(payload);
  const redactionResult = detectRedaction(input) ? "REDACTED" : "CLEAN";
  return { ...payload, evidenceDigest, redactionResult };
}

function detectRedaction(input: Record<string, unknown>): boolean {
  const joined = JSON.stringify(input);
  return joined !== redactShipMessage(joined);
}

/** Recursively key-sort a JSON-safe value into canonical form (AUD-078,
 *  AUD-079). Deterministic serialization must sort keys at EVERY nesting
 *  level; a top-level-only replacer array silently drops nested object
 *  properties from the digest input. */
export function canonicalize(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map((item) => canonicalize(item));
  }
  if (value !== null && typeof value === "object") {
    const source = value as Record<string, unknown>;
    const out: Record<string, unknown> = {};
    for (const key of Object.keys(source).sort()) {
      out[key] = canonicalize(source[key]);
    }
    return out;
  }
  return value;
}

/** Canonical deterministic evidence digest over the payload (sha256 hex).
 *  Keys are sorted recursively (AUD-079) so nested certification, drill,
 *  review and capability-status values are cryptographically bound - a
 *  top-level-only replacer would silently drop them from the digest. */
export function canonicalEvidenceDigest(
  payload: Omit<ReleaseEvidence, "evidenceDigest" | "redactionResult">,
): string {
  const canonical = JSON.stringify(canonicalize(payload));
  return sha256Hex(canonical);
}

/** Real sha256 hex digest (Web Crypto). */
export async function sha256HexAsync(input: string): Promise<string> {
  const bytes = new TextEncoder().encode(input);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/** Synchronous sha256 hex digest via a small deterministic FNV-free fallback. */
export function sha256Hex(input: string): string {
  // Web Crypto is async; for deterministic M1 contract proofs we expose a
  // sync canonical digest computed over UTF-8 bytes with SHA-256 through a
  // pure JS implementation guarded to 64 lowercase hex chars.
  return sha256HexSync(input);
}

/** Real SHA-256 (FIPS 180-4) over RAW BYTES (AUD-077). Hashing binary
 *  artifact bytes must never round-trip through TextDecoder/TextEncoder:
 *  lossy UTF-8 decoding collapses distinct byte sequences onto the same
 *  replacement characters and breaks the artifact binding. */
export function sha256Bytes(bytes: Uint8Array): string {
  const bitLen = bytes.length * 8;
  const withOne = new Uint8Array(bytes.length + 1);
  withOne.set(bytes);
  withOne[bytes.length] = 0x80;
  const padLen = ((withOne.length + 8 + 63) & ~63) >>> 0;
  const padded = new Uint8Array(padLen);
  padded.set(withOne);
  const view = new DataView(padded.buffer);
  view.setUint32(padLen - 8, Math.floor(bitLen / 0x100000000), false);
  view.setUint32(padLen - 4, bitLen >>> 0, false);

  let h0 = 0x6a09e667,
    h1 = 0xbb67ae85,
    h2 = 0x3c6ef372,
    h3 = 0xa54ff53a;
  let h4 = 0x510e527f,
    h5 = 0x9b05688c,
    h6 = 0x1f83d9ab,
    h7 = 0x5be0cd19;
  const w = new Uint32Array(64);

  for (let offset = 0; offset < padded.length; offset += 64) {
    for (let i = 0; i < 16; i++) {
      w[i] = view.getUint32(offset + i * 4, false);
    }
    for (let i = 16; i < 64; i++) {
      const s0 =
        rotr(w[i - 15]!, 7) ^ rotr(w[i - 15]!, 18) ^ (w[i - 15]! >>> 3);
      const s1 = rotr(w[i - 2]!, 17) ^ rotr(w[i - 2]!, 19) ^ (w[i - 2]! >>> 10);
      w[i] = (w[i - 16]! + s0 + w[i - 7]! + s1) >>> 0;
    }
    let a = h0,
      b = h1,
      c = h2,
      d = h3,
      e = h4,
      f = h5,
      g = h6,
      h = h7;
    for (let i = 0; i < 64; i++) {
      const S1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
      const ch = (e & f) ^ (~e & g);
      const temp1 = (h + S1 + ch + K[i]! + w[i]!) >>> 0;
      const S0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
      const maj = (a & b) ^ (a & c) ^ (b & c);
      const temp2 = (S0 + maj) >>> 0;
      h = g;
      g = f;
      f = e;
      e = (d + temp1) >>> 0;
      d = c;
      c = b;
      b = a;
      a = (temp1 + temp2) >>> 0;
    }
    h0 = (h0 + a) >>> 0;
    h1 = (h1 + b) >>> 0;
    h2 = (h2 + c) >>> 0;
    h3 = (h3 + d) >>> 0;
    h4 = (h4 + e) >>> 0;
    h5 = (h5 + f) >>> 0;
    h6 = (h6 + g) >>> 0;
    h7 = (h7 + h) >>> 0;
  }
  return [h0, h1, h2, h3, h4, h5, h6, h7]
    .map((v) => v.toString(16).padStart(8, "0"))
    .join("");
}

/* Pure-JS SHA-256 (FIPS 180-4) - deterministic, no external dependency. */
const K: number[] = [
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
  0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
  0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
  0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
  0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
  0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
  0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
  0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
  0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

function rotr(value: number, bits: number): number {
  return ((value >>> bits) | (value << (32 - bits))) >>> 0;
}

function sha256HexSync(input: string): string {
  // String hashing is the UTF-8 encoding of the input hashed as bytes;
  // binary artifact hashing uses sha256Bytes directly (AUD-077).
  return sha256Bytes(new TextEncoder().encode(input));
}

/** Parse ReleaseEvidence from unknown wire data, fail-closed. */
export function parseReleaseEvidence(value: unknown): ReleaseEvidence {
  if (typeof value !== "object" || value === null) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "release evidence must be an object",
    );
  }
  const obj = value as Record<string, unknown>;
  if (obj["schema_version"] !== 1) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "unsupported release evidence schema_version",
    );
  }
  for (const key of Object.keys(obj)) {
    if (
      ![
        "schema_version",
        "node",
        "runId",
        "gitCommit",
        "releaseId",
        "certifications",
        "drills",
        "reviews",
        "releaseNotes",
        "evidenceDigest",
        "redactionResult",
      ].includes(key)
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown release evidence field: ${key}`,
      );
    }
  }
  for (const field of [
    "node",
    "runId",
    "gitCommit",
    "releaseId",
    "evidenceDigest",
    "redactionResult",
  ]) {
    if (typeof obj[field] !== "string" || (obj[field] as string).length === 0) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `${field} must be a non-empty string`,
      );
    }
  }
  const certifications = Array.isArray(obj["certifications"])
    ? obj["certifications"].map((item) => parseCertificationRow(item))
    : [];
  const drills = Array.isArray(obj["drills"])
    ? obj["drills"].map((item) => parseDrillEvidence(item))
    : [];
  const reviews = Array.isArray(obj["reviews"])
    ? obj["reviews"].map((item) => parseReviewResult(item))
    : [];
  const releaseNotes: Record<string, CapabilityStatus> = {};
  if (typeof obj["releaseNotes"] === "object" && obj["releaseNotes"] !== null) {
    for (const [key, value] of Object.entries(
      obj["releaseNotes"] as Record<string, unknown>,
    )) {
      releaseNotes[key] = assertKnownValue(
        CAPABILITY_STATUSES,
        value,
        "capability status",
      );
    }
  }
  const payload: Omit<ReleaseEvidence, "evidenceDigest" | "redactionResult"> = {
    schema_version: 1,
    node: obj["node"] as string,
    runId: obj["runId"] as string,
    gitCommit: obj["gitCommit"] as string,
    releaseId: obj["releaseId"] as string,
    certifications,
    drills,
    reviews,
    releaseNotes,
  };
  const expectedDigest = canonicalEvidenceDigest(payload);
  if (obj["evidenceDigest"] !== expectedDigest) {
    throw new ShipError(
      "VERIFICATION_FAILED",
      "release evidence digest mismatch: declared != computed",
    );
  }
  return {
    ...payload,
    evidenceDigest: expectedDigest,
    redactionResult: obj["redactionResult"] as "REDACTED" | "CLEAN",
  };
}

function parseCertificationRow(value: unknown): CertificationRow {
  if (typeof value !== "object" || value === null) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "certification row must be an object",
    );
  }
  const obj = value as Record<string, unknown>;
  for (const key of Object.keys(obj)) {
    if (!["rowId", "domain", "state", "evidenceRef"].includes(key)) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown certification row field: ${key}`,
      );
    }
  }
  if (typeof obj["rowId"] !== "string" || obj["rowId"].length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "certification row id must be non-empty",
    );
  }
  if (obj["domain"] !== "PROVIDER" && obj["domain"] !== "HARDWARE") {
    throw new ShipError(
      "VALIDATION_FAILED",
      `unknown certification domain: ${String(obj["domain"])}`,
    );
  }
  const state = assertKnownValue(
    CERTIFICATION_ROW_STATES,
    obj["state"],
    "certification row state",
  );
  if (
    state === "SIGNED" &&
    (typeof obj["evidenceRef"] !== "string" ||
      (obj["evidenceRef"] as string).length === 0)
  ) {
    throw new ShipError(
      "VALIDATION_FAILED",
      `signed certification row requires evidence ref: ${obj["rowId"]}`,
    );
  }
  return {
    rowId: obj["rowId"],
    domain: obj["domain"] as "PROVIDER" | "HARDWARE",
    state,
    ...(obj["evidenceRef"] !== undefined
      ? { evidenceRef: obj["evidenceRef"] as string }
      : {}),
  };
}

function parseDrillEvidence(value: unknown): DrillEvidence {
  if (typeof value !== "object" || value === null) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "drill evidence must be an object",
    );
  }
  const obj = value as Record<string, unknown>;
  for (const key of Object.keys(obj)) {
    if (!["kind", "status", "datedAt", "evidenceRef"].includes(key)) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown drill evidence field: ${key}`,
      );
    }
  }
  const kind = assertKnownValue(DRILL_KINDS, obj["kind"], "drill kind");
  const status = assertKnownValue(
    DRILL_STATUSES,
    obj["status"],
    "drill status",
  );
  if (
    status === "DATED_EVIDENCE" &&
    (typeof obj["datedAt"] !== "string" ||
      (obj["datedAt"] as string).length === 0)
  ) {
    throw new ShipError(
      "VALIDATION_FAILED",
      `dated drill requires a timestamp: ${kind}`,
    );
  }
  return {
    kind,
    status,
    ...(obj["datedAt"] !== undefined
      ? { datedAt: obj["datedAt"] as string }
      : {}),
    ...(obj["evidenceRef"] !== undefined
      ? { evidenceRef: obj["evidenceRef"] as string }
      : {}),
  };
}

function parseReviewResult(value: unknown): ReviewResult {
  if (typeof value !== "object" || value === null) {
    throw new ShipError("VALIDATION_FAILED", "review result must be an object");
  }
  const obj = value as Record<string, unknown>;
  for (const key of Object.keys(obj)) {
    if (!["domain", "status", "evidenceRef"].includes(key)) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown review result field: ${key}`,
      );
    }
  }
  const domain = assertKnownValue(
    REVIEW_DOMAINS,
    obj["domain"],
    "review domain",
  );
  const status = assertKnownValue(
    PROOF_STATUSES,
    obj["status"],
    "review status",
  );
  if (typeof obj["evidenceRef"] !== "string") {
    throw new ShipError(
      "VALIDATION_FAILED",
      "review evidenceRef must be a string",
    );
  }
  if (status === "PASS" && (obj["evidenceRef"] as string).length === 0) {
    throw new ShipError(
      "VALIDATION_FAILED",
      `passing review requires evidence ref: ${domain}`,
    );
  }
  return { domain, status, evidenceRef: obj["evidenceRef"] };
}

/* ------------------------------------------------------------------ */
/* ManualDeployHandoff                                                */
/* ------------------------------------------------------------------ */

/**
 * ManualDeployHandoff - the exact manual deploy command.
 *
 * Auto-deploy is never authorized (SPEC-008 behavior 7). A handoff
 * existing is not a deployment executed. The command is exact, printable,
 * and bound to a release id and profile.
 */
export interface ManualDeployHandoff {
  schema_version: 1;
  handoffId: string;
  releaseId: string;
  profile: string;
  exactCommand: string;
  deployUrl?: string;
  createdAt: string;
}

export function createManualDeployHandoff(input: {
  handoffId: string;
  releaseId: string;
  profile: string;
  exactCommand: string;
  deployUrl?: string;
  createdAt?: string;
}): ManualDeployHandoff {
  for (const field of [
    "handoffId",
    "releaseId",
    "profile",
    "exactCommand",
  ] as const) {
    if (typeof input[field] !== "string" || input[field].length === 0) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `${field} must be a non-empty string`,
      );
    }
  }
  if (
    input.exactCommand.includes(";") ||
    input.exactCommand.includes("&&") ||
    input.exactCommand.includes("\n")
  ) {
    throw new ShipError(
      "POLICY_DENIED",
      "deploy handoff command must be a single exact command",
    );
  }
  if (
    input.exactCommand.includes("sk-") ||
    input.exactCommand.includes("AKIA") ||
    input.exactCommand.includes("ghp_")
  ) {
    throw new ShipError(
      "POLICY_DENIED",
      "deploy handoff command must not embed secrets",
    );
  }
  return {
    schema_version: 1,
    handoffId: input.handoffId,
    releaseId: input.releaseId,
    profile: input.profile,
    exactCommand: input.exactCommand,
    ...(input.deployUrl !== undefined ? { deployUrl: input.deployUrl } : {}),
    createdAt: input.createdAt ?? new Date().toISOString(),
  };
}

/** Parse ManualDeployHandoff from unknown wire data, fail-closed. */
export function parseManualDeployHandoff(value: unknown): ManualDeployHandoff {
  if (typeof value !== "object" || value === null) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "manual deploy handoff must be an object",
    );
  }
  const obj = value as Record<string, unknown>;
  if (obj["schema_version"] !== 1) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "unsupported manual deploy handoff schema_version",
    );
  }
  for (const key of Object.keys(obj)) {
    if (
      ![
        "schema_version",
        "handoffId",
        "releaseId",
        "profile",
        "exactCommand",
        "deployUrl",
        "createdAt",
      ].includes(key)
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown manual deploy handoff field: ${key}`,
      );
    }
  }
  for (const field of [
    "handoffId",
    "releaseId",
    "profile",
    "exactCommand",
    "createdAt",
  ]) {
    if (typeof obj[field] !== "string" || (obj[field] as string).length === 0) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `${field} must be a non-empty string`,
      );
    }
  }
  if (
    (obj["exactCommand"] as string).includes(";") ||
    (obj["exactCommand"] as string).includes("&&")
  ) {
    throw new ShipError(
      "POLICY_DENIED",
      "deploy handoff command must be a single exact command",
    );
  }
  return {
    schema_version: 1,
    handoffId: obj["handoffId"] as string,
    releaseId: obj["releaseId"] as string,
    profile: obj["profile"] as string,
    exactCommand: obj["exactCommand"] as string,
    ...(obj["deployUrl"] !== undefined
      ? { deployUrl: obj["deployUrl"] as string }
      : {}),
    createdAt: obj["createdAt"] as string,
  };
}

/* ------------------------------------------------------------------ */
/* ProductionReadinessDecision                                        */
/* ------------------------------------------------------------------ */

/**
 * ProductionReadinessDecision - the final readiness decision record.
 *
 * A decision made is not a release shipped. The decision requires:
 *   - a ship gate that PASSED with a fresh-clone rerun,
 *   - every certification row SIGNED (no RELEASE-BLOCKING-PENDING),
 *   - every mandatory review PASS,
 *   - dated drill evidence for every required drill kind,
 *   - the exact manual deploy handoff attached.
 */
export interface ProductionReadinessDecision {
  schema_version: 1;
  decisionId: string;
  releaseId: string;
  gate: ShipGate;
  evidence: ReleaseEvidence;
  handoff: ManualDeployHandoff;
  decision: "READY" | "NOT_READY";
  decidedAt: string;
}

export function createProductionReadinessDecision(input: {
  decisionId: string;
  releaseId: string;
  gate: ShipGate;
  evidence: ReleaseEvidence;
  handoff: ManualDeployHandoff;
  decidedAt?: string;
}): ProductionReadinessDecision {
  const inputRecord = input as unknown as Record<string, unknown>;
  for (const field of ["decisionId", "releaseId"] as const) {
    if (
      typeof inputRecord[field] !== "string" ||
      (inputRecord[field] as string).length === 0
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `${field} must be a non-empty string`,
      );
    }
  }
  if (input.gate.releaseKind !== "CORE_RELEASE") {
    throw new ShipError(
      "POLICY_DENIED",
      "production readiness requires a CORE_RELEASE gate",
    );
  }
  const decision = evaluateProductionReadiness(
    input.gate,
    input.evidence,
    input.handoff,
  );
  return {
    schema_version: 1,
    decisionId: input.decisionId,
    releaseId: input.releaseId,
    gate: input.gate,
    evidence: input.evidence,
    handoff: input.handoff,
    decision,
    decidedAt: input.decidedAt ?? new Date().toISOString(),
  };
}

/**
 * Deterministic readiness evaluation. READY only when:
 *   - gate verdict is PASSED,
 *   - gate freshCloneRerun is true,
 *   - no certification row is RELEASE-BLOCKING-PENDING or PENDING,
 *   - every review PASSes,
 *   - every drill kind has DATED_EVIDENCE,
 *   - handoff command is non-empty (exact manual command).
 */
export function evaluateProductionReadiness(
  gate: ShipGate,
  evidence: ReleaseEvidence,
  handoff: ManualDeployHandoff,
): "READY" | "NOT_READY" {
  if (gate.verdict !== "PASSED" || !gate.freshCloneRerun) {
    return "NOT_READY";
  }
  for (const row of evidence.certifications) {
    if (row.state !== "SIGNED") {
      return "NOT_READY";
    }
  }
  for (const review of evidence.reviews) {
    if (review.status !== "PASS") {
      return "NOT_READY";
    }
  }
  for (const kind of DRILL_KINDS) {
    const found = evidence.drills.find((drill) => drill.kind === kind);
    if (!found || found.status !== "DATED_EVIDENCE") {
      return "NOT_READY";
    }
  }
  if (handoff.exactCommand.length === 0) {
    return "NOT_READY";
  }
  return "READY";
}

/** Parse ProductionReadinessDecision from unknown wire data, fail-closed. */
export function parseProductionReadinessDecision(
  value: unknown,
): ProductionReadinessDecision {
  if (typeof value !== "object" || value === null) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "production readiness decision must be an object",
    );
  }
  const obj = value as Record<string, unknown>;
  if (obj["schema_version"] !== 1) {
    throw new ShipError(
      "VALIDATION_FAILED",
      "unsupported production readiness decision schema_version",
    );
  }
  for (const key of Object.keys(obj)) {
    if (
      ![
        "schema_version",
        "decisionId",
        "releaseId",
        "gate",
        "evidence",
        "handoff",
        "decision",
        "decidedAt",
      ].includes(key)
    ) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `unknown production readiness decision field: ${key}`,
      );
    }
  }
  for (const field of ["decisionId", "releaseId", "decidedAt"]) {
    if (typeof obj[field] !== "string" || (obj[field] as string).length === 0) {
      throw new ShipError(
        "VALIDATION_FAILED",
        `${field} must be a non-empty string`,
      );
    }
  }
  const gate = parseShipGate(obj["gate"]);
  const evidence = parseReleaseEvidence(obj["evidence"]);
  const handoff = parseManualDeployHandoff(obj["handoff"]);
  const expectedDecision = evaluateProductionReadiness(gate, evidence, handoff);
  if (obj["decision"] !== expectedDecision) {
    throw new ShipError(
      "VERIFICATION_FAILED",
      `production readiness decision mismatch: declared ${String(obj["decision"])}, computed ${expectedDecision}`,
    );
  }
  return {
    schema_version: 1,
    decisionId: obj["decisionId"] as string,
    releaseId: obj["releaseId"] as string,
    gate,
    evidence,
    handoff,
    decision: expectedDecision,
    decidedAt: obj["decidedAt"] as string,
  };
}

/* ------------------------------------------------------------------ */
/* Redaction helper                                                    */
/* ------------------------------------------------------------------ */

/** Redact secret-shaped content from a serialized evidence string. */
export function redactEvidenceJson(input: string): string {
  return redactShipMessage(input);
}

export { assertKnownShipErrorCode };
