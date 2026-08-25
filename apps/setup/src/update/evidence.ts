/**
 * EP-042 M2 current-run redacted release/update evidence (SPEC-016,
 * SPEC-024).
 *
 * Binds run_id, git_commit, release identity, manifest digest,
 * component identities/digests, compatibility decision, update plan
 * digest, backup/rollback state, promotion state, final decision, and a
 * redaction result. Secret-shaped values are redacted at the boundary;
 * raw secrets are never emitted.
 */

import { ReleaseError, ReleaseErrorCode } from "./errors";
import type { Digest } from "./types";
import type { CompatibilityMatrix, ReleaseManifest, UpdatePlan } from "./types";

export interface RedactedEvidenceInput {
  run_id: string;
  git_commit: string;
  release: ReleaseManifest;
  manifest_digest: Digest;
  compatibility_decision: "COMPATIBLE" | "INCOMPATIBLE";
  update_plan: UpdatePlan;
  update_plan_digest: Digest;
  backup_state: "REQUESTED" | "COMPLETED" | "DENIED";
  rollback_state: "PROVEN" | "NOT_PROVEN" | "DENIED";
  promotion_state:
    | "LOCKED"
    | "AWAITING_HUMAN_APPROVAL"
    | "APPROVED_MANUAL_ONLY";
  final_decision: "DENY" | "APPROVE_MANUAL";
  created_at: string;
  redaction_canary?: string;
}

export interface RedactedEvidence {
  run_id: string;
  git_commit: string;
  release_id: string;
  release_version: string;
  manifest_digest: string;
  component_identities: ReadonlyArray<string>;
  component_digests: ReadonlyArray<string>;
  compatibility_decision: "COMPATIBLE" | "INCOMPATIBLE";
  update_plan_id: string;
  update_plan_digest: string;
  backup_state: "REQUESTED" | "COMPLETED" | "DENIED";
  rollback_state: "PROVEN" | "NOT_PROVEN" | "DENIED";
  promotion_state:
    | "LOCKED"
    | "AWAITING_HUMAN_APPROVAL"
    | "APPROVED_MANUAL_ONLY";
  final_decision: "DENY" | "APPROVE_MANUAL";
  redaction_applied: boolean;
  created_at: string;
}

const SECRET_SHAPED_PATTERNS: ReadonlyArray<RegExp> = [
  /sk-[A-Za-z0-9_-]{8,}/,
  /AKIA[0-9A-Z]{16}/,
  /-----BEGIN [A-Z ]*PRIVATE KEY-----/,
  /xox[baprs]-[A-Za-z0-9-]{10,}/,
  /ghp_[A-Za-z0-9]{20,}/,
  /Bearer [A-Za-z0-9._-]{20,}/i,
];

function looksSecretShaped(value: string): boolean {
  return SECRET_SHAPED_PATTERNS.some((pattern) => pattern.test(value));
}

/**
 * Redact a single value: secret-shaped strings become [REDACTED].
 * Redaction is applied to every evidence field at the boundary.
 */
export function redactValue(value: string): string {
  return looksSecretShaped(value) ? "[REDACTED]" : value;
}

/**
 * Build current-run redacted evidence. Every input value passes through
 * the redaction boundary; the evidence record never carries raw
 * secret-shaped content. A caller-supplied canary (runtime-constructed
 * in tests) must never appear unredacted.
 */
export function buildRedactedEvidence(
  input: RedactedEvidenceInput,
): RedactedEvidence {
  const componentIdentities = input.release.components.map((component) =>
    redactValue(component.component_id),
  );
  const componentDigests = input.release.components.map((component) =>
    redactValue(component.digest),
  );

  const evidence: RedactedEvidence = {
    run_id: redactValue(input.run_id),
    git_commit: redactValue(input.git_commit),
    release_id: redactValue(input.release.release_id),
    release_version: redactValue(input.release.version),
    manifest_digest: redactValue(input.manifest_digest.asString()),
    component_identities: componentIdentities,
    component_digests: componentDigests,
    compatibility_decision: input.compatibility_decision,
    update_plan_id: redactValue(input.update_plan.plan_id),
    update_plan_digest: redactValue(input.update_plan_digest.asString()),
    backup_state: input.backup_state,
    rollback_state: input.rollback_state,
    promotion_state: input.promotion_state,
    final_decision: input.final_decision,
    redaction_applied: false,
    created_at: redactValue(input.created_at),
  };

  // A runtime-constructed canary must never leak unredacted.
  let redactionApplied = false;
  if (input.redaction_canary !== undefined) {
    const redacted = redactValue(input.redaction_canary);
    redactionApplied = redacted === "[REDACTED]";
  }
  evidence.redaction_applied = redactionApplied;

  for (const value of Object.values(evidence)) {
    if (typeof value === "string" && looksSecretShaped(value)) {
      throw new ReleaseError(
        ReleaseErrorCode.InternalInvariant,
        "evidence contains unredacted secret-shaped content",
      );
    }
  }
  for (const value of componentIdentities) {
    if (looksSecretShaped(value)) {
      throw new ReleaseError(
        ReleaseErrorCode.InternalInvariant,
        "evidence component identity contains unredacted secret-shaped content",
      );
    }
  }
  for (const value of componentDigests) {
    if (looksSecretShaped(value)) {
      throw new ReleaseError(
        ReleaseErrorCode.InternalInvariant,
        "evidence component digest contains unredacted secret-shaped content",
      );
    }
  }

  return evidence;
}

/**
 * Compatibility decision helper: produce the evidence vocabulary value
 * from a verdict.
 */
export function compatibilityDecisionLabel(
  compatible: boolean,
): "COMPATIBLE" | "INCOMPATIBLE" {
  return compatible ? "COMPATIBLE" : "INCOMPATIBLE";
}

/**
 * Plan digest binding for evidence: the declared plan digest must equal
 * the computed digest; mismatch is denied.
 */
export function planDigestMatches(
  plan: UpdatePlan,
  declared: Digest,
  computed: Digest,
): boolean {
  void plan;
  return declared.equals(computed);
}
