/**
 * EP-016 memory workflow vocabulary (SPEC-002 requirement 8; ADR-023).
 *
 * SPEC-002 locks the canonical terms MemoryRecord, MemoryProposal,
 * MemoryType, Sensitivity, RetentionPolicy, Provenance, Supersession,
 * EmbeddingRef, WorldGraphRepository, ContextCandidate, and
 * ContextCapsule. This module adds the EP-016-owned durable workflow
 * vocabulary: the memory workflow kinds and memory operation kinds the
 * memory plane's durable workflows are built from.
 *
 * Every enum here is vocabulary locked: unknown values are rejected at
 * parse time, and a new synonym requires an ADR and schema update
 * (ADR-023).
 */

import { WorkflowContractError } from "../errors.js";

export interface MemoryVocabulary<T extends string> {
  readonly name: string;
  readonly values: readonly T[];
  is(value: unknown): value is T;
  parse(value: unknown, context?: string): T;
}

export function defineMemoryVocabulary<T extends string>(
  name: string,
  values: readonly T[],
): MemoryVocabulary<T> {
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
 * Memory workflow kinds (SPEC-002 requirement 8; ADR-023). Each kind is
 * a durable, audited workflow over the memory plane. Kinds are
 * EP-016-owned; they are distinct from the EP-006 workflow kinds and are
 * never mixed with them at the registry boundary.
 */
export const memoryWorkflowKind = defineMemoryVocabulary("MemoryWorkflowKind", [
  "MEMORY_CONSOLIDATION",
  "MEMORY_RETENTION",
  "MEMORY_LEGAL_HOLD",
  "MEMORY_EXPORT",
  "MEMORY_DELETION",
  "MEMORY_REEMBED",
] as const);
export type MemoryWorkflowKind =
  (typeof memoryWorkflowKind.values)[number];

/**
 * Memory operation kinds: the durable activity-level operations the
 * memory workflows schedule. Each maps to a bounded, idempotent,
 * error-classified activity (SPEC-006 behavior 7). Kinds are
 * EP-016-owned.
 */
export const memoryOperationKind = defineMemoryVocabulary("MemoryOperationKind", [
  "PROPOSE",
  "EVALUATE_PROPOSAL",
  "ACTIVATE_CANONICAL",
  "SUPERSEDE",
  "RETENTION_SWEEP",
  "LEGAL_HOLD_APPLY",
  "LEGAL_HOLD_RELEASE",
  "EXPORT_SNAPSHOT",
  "DELETE_RECORD",
  "REEMBED",
] as const);
export type MemoryOperationKind =
  (typeof memoryOperationKind.values)[number];

/**
 * Memory workflow lifecycle states. Terminal outcomes mirror SPEC-006
 * ActionLifecycle; durable engine states CANCELLED and TIMED_OUT are
 * explicit (SPEC-023 behavior 5).
 */
export const memoryWorkflowState = defineMemoryVocabulary("MemoryWorkflowState", [
  "REQUESTED",
  "EVALUATING",
  "AWAITING_APPROVAL",
  "EXECUTING",
  "VERIFYING",
  "SUCCEEDED",
  "FAILED",
  "CANCELLED",
  "TIMED_OUT",
] as const);
export type MemoryWorkflowState =
  (typeof memoryWorkflowState.values)[number];

/**
 * Legal hold decision (SPEC-002 requirement 8). APPLY freezes a record
 * against retention deletion; RELEASE restores normal retention. A legal
 * hold preserves storage; it never implies context relevance.
 */
export const legalHoldDecision = defineMemoryVocabulary("LegalHoldDecision", [
  "APPLY",
  "RELEASE",
] as const);
export type LegalHoldDecision = (typeof legalHoldDecision.values)[number];

/** Retention disposition decided by a sweep (SPEC-002 requirement 8). */
export const retentionDisposition = defineMemoryVocabulary("RetentionDisposition", [
  "KEEP",
  "DELETE",
  "LEGAL_HOLD",
] as const);
export type RetentionDisposition =
  (typeof retentionDisposition.values)[number];

export const MEMORY_WORKFLOW_VOCABULARY_VERSION = "1.0.0";
