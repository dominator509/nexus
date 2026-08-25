/**
 * EP-042 M4 installer observability (SPEC-016, SPEC-024; fence M).
 *
 * Current-run redacted installer evidence exposing exact states:
 * release id, install id, run_id, git_commit, manifest digest,
 * component digests, backup state, staging state, compatibility
 * state, install state, rollback state, failure class, recovery
 * state, cleanup state, and a redaction result.
 *
 * RECOVERED is only recorded after recovery verification actually
 * succeeded; OBSERVABILITY EVENT EXISTS != RECOVERY PROVEN.
 *
 * No raw secrets: redaction is applied at the boundary with
 * runtime-constructed canaries; no tracked secret literals.
 */

import { InstallerError } from "./errors";

export interface InstallerEvidenceInput {
  run_id: string;
  git_commit: string;
  install_id: string;
  release_id: string;
  manifest_digest: string;
  component_identities: ReadonlyArray<string>;
  component_digests: ReadonlyArray<string>;
  compatibility_state: "COMPATIBLE" | "INCOMPATIBLE" | "NOT_EVALUATED";
  backup_state: "REQUESTED" | "COMPLETED" | "FAILED" | "NOT_REQUIRED";
  staging_state: "NONE" | "STAGING" | "STAGED" | "VALIDATED" | "FAILED";
  install_state: "NONE" | "INSTALLED" | "FAILED" | "ROLLED_BACK";
  rollback_state: "NONE" | "REQUIRED" | "COMPLETED" | "FAILED";
  failure_class?: string;
  recovery_state: "NONE" | "REQUIRED" | "COMPLETED" | "FAILED";
  cleanup_state: "NONE" | "CLEANED" | "LEFTOVER";
  redaction_canary?: string;
  created_at: string;
}

export interface InstallerEvidence {
  run_id: string;
  git_commit: string;
  install_id: string;
  release_id: string;
  manifest_digest: string;
  component_identities: ReadonlyArray<string>;
  component_digests: ReadonlyArray<string>;
  compatibility_state: "COMPATIBLE" | "INCOMPATIBLE" | "NOT_EVALUATED";
  backup_state: "REQUESTED" | "COMPLETED" | "FAILED" | "NOT_REQUIRED";
  staging_state: "NONE" | "STAGING" | "STAGED" | "VALIDATED" | "FAILED";
  install_state: "NONE" | "INSTALLED" | "FAILED" | "ROLLED_BACK";
  rollback_state: "NONE" | "REQUIRED" | "COMPLETED" | "FAILED";
  failure_class?: string;
  recovery_state: "NONE" | "REQUIRED" | "COMPLETED" | "FAILED";
  cleanup_state: "NONE" | "CLEANED" | "LEFTOVER";
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

export function looksSecretShaped(value: string): boolean {
  return SECRET_SHAPED_PATTERNS.some((pattern) => pattern.test(value));
}

/** Redact a single value: secret-shaped strings become [REDACTED]. */
export function redactValue(value: string): string {
  return looksSecretShaped(value) ? "[REDACTED]" : value;
}

/**
 * Build current-run redacted installer evidence. Every input value
 * passes through the redaction boundary; a caller-supplied canary
 * (runtime-constructed in tests) must never appear unredacted.
 */
export function buildInstallerEvidence(
  input: InstallerEvidenceInput,
): InstallerEvidence {
  const evidence: InstallerEvidence = {
    run_id: redactValue(input.run_id),
    git_commit: redactValue(input.git_commit),
    install_id: redactValue(input.install_id),
    release_id: redactValue(input.release_id),
    manifest_digest: redactValue(input.manifest_digest),
    component_identities: input.component_identities.map((id) =>
      redactValue(id),
    ),
    component_digests: input.component_digests.map((d) => redactValue(d)),
    compatibility_state: input.compatibility_state,
    backup_state: input.backup_state,
    staging_state: input.staging_state,
    install_state: input.install_state,
    rollback_state: input.rollback_state,
    recovery_state: input.recovery_state,
    cleanup_state: input.cleanup_state,
    redaction_applied: false,
    created_at: redactValue(input.created_at),
  };
  if (input.failure_class !== undefined) {
    evidence.failure_class = redactValue(input.failure_class);
  }

  let redactionApplied = false;
  if (input.redaction_canary !== undefined) {
    redactionApplied = redactValue(input.redaction_canary) === "[REDACTED]";
  }
  evidence.redaction_applied = redactionApplied;

  for (const value of Object.values(evidence)) {
    if (typeof value === "string" && looksSecretShaped(value)) {
      throw new InstallerError(
        "VALIDATION_FAILED",
        "installer evidence contains unredacted secret-shaped content",
      );
    }
  }
  return evidence;
}

/** Fresh installer evidence with no operation recorded yet. */
export function emptyInstallerEvidence(input: {
  run_id: string;
  git_commit: string;
  install_id: string;
  release_id: string;
  manifest_digest: string;
  created_at: string;
}): InstallerEvidence {
  return {
    run_id: redactValue(input.run_id),
    git_commit: redactValue(input.git_commit),
    install_id: redactValue(input.install_id),
    release_id: redactValue(input.release_id),
    manifest_digest: redactValue(input.manifest_digest),
    component_identities: [],
    component_digests: [],
    compatibility_state: "NOT_EVALUATED",
    backup_state: "NOT_REQUIRED",
    staging_state: "NONE",
    install_state: "NONE",
    rollback_state: "NONE",
    recovery_state: "NONE",
    cleanup_state: "NONE",
    redaction_applied: false,
    created_at: redactValue(input.created_at),
  };
}
