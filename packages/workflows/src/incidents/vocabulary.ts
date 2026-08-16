/**
 * EP-019 incident workflow vocabulary (SPEC-018; ADR-026).
 *
 * SPEC-018 locks the canonical terms Incident, Diagnosis, Reproduction,
 * PatchCandidate, Review, Canary, HealthCriterion, Rollback,
 * SkillCandidate, and IncidentMemory. This module adds the
 * EP-019-owned durable workflow vocabulary: the incident remediation
 * workflow kinds and operation kinds the self-healing plane's durable
 * workflows are built from.
 *
 * The canonical lifecycle is OBSERVE -> INCIDENT -> CORRELATE ->
 * DIAGNOSE -> REPRODUCE -> PATCH_PROPOSED -> SANDBOX_VALIDATION ->
 * SECURITY_VALIDATION -> APPROVAL -> STAGED_DEPLOYMENT ->
 * POST_DEPLOY_VERIFICATION -> CLOSED, with explicit terminal/failure
 * states. No state may be collapsed and no model/agent may declare its
 * own fix successful.
 *
 * Every enum here is vocabulary locked: unknown values are rejected at
 * parse time, and a new synonym requires an ADR and schema update
 * (ADR-026).
 */

import { WorkflowContractError } from "../errors.js";

export interface IncidentsVocabulary<T extends string> {
  readonly name: string;
  readonly values: readonly T[];
  is(value: unknown): value is T;
  parse(value: unknown, context?: string): T;
}

export function defineIncidentsVocabulary<T extends string>(
  name: string,
  values: readonly T[],
): IncidentsVocabulary<T> {
  const set = new Set<string>(values);
  return {
    name,
    values,
    is(value: unknown): value is T {
      return typeof value === "string" && set.has(value);
    },
    parse(value: unknown, context = name): T {
      if (typeof value === "string" && set.has(value)) {
        return value as T;
      }
      throw new WorkflowContractError(
        `invalid ${context}: ${JSON.stringify(value)}; expected one of ${values.join(", ")}`,
      );
    },
  };
}

/**
 * Incident remediation workflow kinds (SPEC-018; ADR-026). Each kind is
 * a durable, audited workflow over the self-healing plane. Kinds are
 * EP-019-owned; they are distinct from the EP-006 workflow kinds and
 * are never mixed with them at the registry boundary.
 */
export const incidentWorkflowKind = defineIncidentsVocabulary(
  "IncidentWorkflowKind",
  [
    "INCIDENT_LIFECYCLE",
    "DIAGNOSIS",
    "REPRODUCTION",
    "PATCH_PROPOSAL",
    "REVIEW",
    "CANARY_DEPLOYMENT",
    "ROLLBACK",
  ] as const,
);
export type IncidentWorkflowKind = (typeof incidentWorkflowKind.values)[number];

/**
 * Incident operation kinds: the durable activity-level operations the
 * incident workflows schedule. Each maps to a bounded, idempotent,
 * error-classified activity (SPEC-006 behavior 7). Kinds are
 * EP-019-owned.
 */
export const incidentOperationKind = defineIncidentsVocabulary(
  "IncidentOperationKind",
  [
    "OBSERVE_SIGNAL",
    "CORRELATE_INCIDENT",
    "CREATE_DIAGNOSIS",
    "RUN_REPRODUCTION",
    "GENERATE_PATCH",
    "VALIDATE_SANDBOX",
    "VALIDATE_SECURITY",
    "REQUEST_APPROVAL",
    "STAGE_DEPLOYMENT",
    "VERIFY_POST_DEPLOY",
    "EXECUTE_ROLLBACK",
    "RECORD_MEMORY",
  ] as const,
);
export type IncidentOperationKind =
  (typeof incidentOperationKind.values)[number];

/**
 * Incident lifecycle states (SPEC-018; ADR-026). The exact canonical
 * lifecycle from the EP-019 owner directive, including explicit
 * terminal/failure states. Serializes as SCREAMING_SNAKE_CASE. There is
 * deliberately NO "FIXED"/"REMEDIATED" value: only real observed
 * verification reaches CLOSED.
 */
export const incidentLifecycleState = defineIncidentsVocabulary(
  "IncidentLifecycleState",
  [
    "OBSERVE",
    "INCIDENT",
    "CORRELATE",
    "DIAGNOSE",
    "REPRODUCE",
    "PATCH_PROPOSED",
    "SANDBOX_VALIDATION",
    "SECURITY_VALIDATION",
    "APPROVAL",
    "STAGED_DEPLOYMENT",
    "POST_DEPLOY_VERIFICATION",
    "CLOSED",
    "REJECTED",
    "UNREPRODUCIBLE",
    "VALIDATION_FAILED",
    "SECURITY_FAILED",
    "ROLLED_BACK",
    "BLOCKED",
  ] as const,
);
export type IncidentLifecycleState =
  (typeof incidentLifecycleState.values)[number];

export const INCIDENT_TERMINAL_STATES: readonly IncidentLifecycleState[] = [
  "CLOSED",
  "REJECTED",
  "UNREPRODUCIBLE",
  "VALIDATION_FAILED",
  "SECURITY_FAILED",
  "ROLLED_BACK",
  "BLOCKED",
] as const;

export function isIncidentTerminal(state: IncidentLifecycleState): boolean {
  return (INCIDENT_TERMINAL_STATES as readonly string[]).includes(state);
}

/**
 * Diagnosis confidence (SPEC-018; ADR-026). A model-generated
 * explanation ALWAYS begins as HYPOTHESIS; only reproducible evidence
 * raises it to VALIDATED.
 */
export const diagnosisConfidence = defineIncidentsVocabulary(
  "DiagnosisConfidence",
  ["HYPOTHESIS", "SUPPORTED", "REPRODUCED", "VALIDATED"] as const,
);
export type DiagnosisConfidence = (typeof diagnosisConfidence.values)[number];

/** Independent review verdict (SPEC-018 behavior 4; ADR-026). */
export const reviewVerdict = defineIncidentsVocabulary(
  "IncidentReviewVerdict",
  ["APPROVE", "REJECT", "REQUEST_CHANGES"] as const,
);
export type IncidentReviewVerdict = (typeof reviewVerdict.values)[number];

/** Canary outcome (SPEC-018; ADR-026). */
export const canaryOutcome = defineIncidentsVocabulary("CanaryOutcome", [
  "HEALTHY",
  "PROMOTED",
  "ROLLED_BACK",
  "FAILED",
] as const);
export type CanaryOutcome = (typeof canaryOutcome.values)[number];

/** Rollback outcome (SPEC-018; ADR-026). */
export const rollbackOutcome = defineIncidentsVocabulary("RollbackOutcome", [
  "RESTORED",
  "FAILED",
] as const);
export type RollbackOutcome = (typeof rollbackOutcome.values)[number];
