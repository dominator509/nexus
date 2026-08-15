/**
 * EP-016 durable memory workflow contracts (SPEC-002 requirement 8;
 * ADR-023).
 *
 * SPEC-002 requires: "Export, deletion, legal hold, retention, and
 * re-embedding workflows are durable and audited." This module defines
 * the six versioned workflow contracts over the memory plane:
 *
 * - MemoryConsolidationWorkflow: proposal-before-canonical. Source
 *   observations are bundled into a MemoryProposal, evaluated by policy,
 *   and only promoted to canonical memory when accepted. A model or
 *   consolidator never writes canonical memory directly (SPEC-002
 *   behavior 5).
 * - MemoryRetentionWorkflow: deterministic retention sweep. Expired
 *   records are deleted unless under legal hold; legal hold never
 *   implies context relevance.
 * - MemoryLegalHoldWorkflow: apply or release a legal hold on a memory
 *   record. Preserves storage; does not surface the record into active
 *   context.
 * - MemoryExportWorkflow: audited, tenant-scoped snapshot export. The
 *   export artifact is produced by an idempotent activity; the workflow
 *   never touches filesystem or network directly.
 * - MemoryDeletionWorkflow: terminal, audited deletion. Requires an
 *   explicit deletion receipt; runs compensation when the delete is not
 *   verifiable.
 * - MemoryReembedWorkflow: re-embedding after a model/embedding schema
 *   change. Each re-embed step is an idempotent activity; failures are
 *   classified and retried within the bounded policy.
 *
 * Hard invariants encoded here (same discipline as EP-006):
 * 1. Workflow code is deterministic: no wall clock, network, database,
 *    filesystem, or random calls. All I/O happens through
 *    scheduleActivity() (SPEC-023 behavior 6).
 * 2. Every workflow is a versioned, named contract with a pinned signal
 *    and query surface. Breaking changes bump the name, never mutate a
 *    live version.
 * 3. Every activity is idempotent (idempotencyRequired) and carries a
 *    bounded, error-classified retry policy; PERMANENT is never retried.
 * 4. Timeout and cancel paths are explicit contracts: execution/run/task
 *    timeouts and cancellation semantics (CANCEL fails closed,
 *    COMPENSATE rolls back).
 */

import type {
  ActivityContract,
  CompensationStep,
  PrincipalRef,
} from "../activities.js";
import type { WorkflowInput } from "../workflows.js";
import type { WorkflowPolicy } from "../policies.js";
import { parseActivityId } from "../ids.js";
import { DEFAULT_RETRY_POLICY } from "../policies.js";
import type { RetryPolicy } from "../policies.js";
import { WORKFLOW_CONTRACT_VERSION } from "../versioning.js";
import type {
  ActivityKind,
  QueryType,
  SignalType,
  WorkflowOutcome,
  WorkflowState,
} from "../vocabulary.js";
import { memoryWorkflowKind } from "./vocabulary.js";
import type {
  LegalHoldDecision,
  MemoryOperationKind,
  MemoryWorkflowKind,
} from "./vocabulary.js";

/** Activity idempotency keys are stable prefixes, never random. */
export const MEMORY_ACTIVITY_IDEMPOTENCY_PREFIX = "memory-op";

const MEMORY_EFFECT_RETRY: RetryPolicy = {
  ...DEFAULT_RETRY_POLICY,
  maxAttempts: 5,
  retryableErrorClasses: ["TRANSIENT", "RATE_LIMIT", "UNAVAILABLE", "TIMEOUT"],
};

const MEMORY_VERIFY_RETRY: RetryPolicy = {
  ...DEFAULT_RETRY_POLICY,
  maxAttempts: 3,
};

/** A bounded memory activity with a stable idempotency key prefix. */
export interface MemoryActivityContract extends ActivityContract {
  readonly operationKind: MemoryOperationKind;
  readonly idempotencyKeyPrefix: string;
}

function memoryActivity(
  activityId: string,
  operationKind: MemoryOperationKind,
  kind: ActivityKind,
  retry: RetryPolicy,
  compensation?: CompensationStep,
): MemoryActivityContract {
  return {
    activityId: parseActivityId(activityId),
    kind,
    operationKind,
    idempotencyRequired: true,
    idempotencyKeyPrefix: `${MEMORY_ACTIVITY_IDEMPOTENCY_PREFIX}:${operationKind.toLowerCase()}`,
    retry,
    timeoutMs: 10 * 60 * 1000,
    ...(compensation === undefined ? {} : { compensation }),
  };
}

const MEMORY_WORKFLOW_POLICY: WorkflowPolicy = {
  timeouts: {
    executionTimeoutMs: 30 * 24 * 60 * 60 * 1000,
    runTimeoutMs: 30 * 24 * 60 * 60 * 1000,
    taskTimeoutMs: 10 * 60 * 1000,
    approvalTimeoutMs: 5 * 24 * 60 * 60 * 1000,
  },
  cancelAction: "COMPENSATE",
  defaultActivityRetry: DEFAULT_RETRY_POLICY,
};

/**
 * Memory workflow spec: the same structural surface as the EP-006
 * WorkflowSpec but pinned to the EP-016 memory workflow kinds and memory
 * activity contracts. EP-016-owned; the engine adapter implements
 * execute() against this surface, exactly as the EP-006 adapter does for
 * its kinds.
 */
export interface MemoryWorkflowSpec<I = unknown, O = unknown> {
  readonly kind: MemoryWorkflowKind;
  /** Stable workflow name; breaking changes bump the name (versioning). */
  readonly name: string;
  /** Semver contract version; see src/versioning.ts. */
  readonly version: string;
  readonly description: string;
  readonly signals: readonly SignalType[];
  readonly queries: readonly QueryType[];
  readonly activities: readonly MemoryActivityContract[];
  readonly policy: WorkflowPolicy;
  readonly execute?: (
    ctx: import("../workflows.js").WorkflowContext,
    input: I,
  ) => Promise<{ state: WorkflowState; outcome?: WorkflowOutcome } & {
    output?: O;
  }>;
}

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

/** Common memory workflow input. */
export interface MemoryWorkflowInput extends WorkflowInput {
  /** Canonical memory record id (SPEC-002 MemoryRecord). */
  readonly memoryId: string;
  /** Canonical memory namespace (SPEC-020; INV-007). */
  readonly namespace: string;
}

export interface MemoryConsolidationInput extends WorkflowInput {
  /** Source record ids bundled into the proposal. */
  readonly sourceMemoryIds: readonly string[];
  /** Target canonical memory type (SPEC-002 MemoryType). */
  readonly targetType: string;
  /** Sensitivity ceiling; the proposal never exceeds the source max. */
  readonly sensitivity: string;
  /** Retention policy for the proposal (SPEC-002 RetentionPolicy). */
  readonly retention: string;
}

export interface MemoryRetentionInput extends WorkflowInput {
  /** Retention window in milliseconds (bounded sweep horizon). */
  readonly sweepHorizonMs: number;
  /** Namespaces to sweep; empty means tenant-wide within permission. */
  readonly namespaces: readonly string[];
}

export interface MemoryLegalHoldInput extends MemoryWorkflowInput {
  readonly decision: LegalHoldDecision;
  readonly reason: string;
}

export interface MemoryExportInput extends WorkflowInput {
  /** Namespace to export; empty means tenant-wide within permission. */
  readonly namespace: string;
  readonly includeSensitive: boolean;
}

export interface MemoryDeletionInput extends MemoryWorkflowInput {
  /** Explicit deletion authorization digest (never free text). */
  readonly deletionDigest: string;
}

export interface MemoryReembedInput extends WorkflowInput {
  /** Embedding schema version target (SPEC-002 EmbeddingRef). */
  readonly targetEmbeddingVersion: string;
  /** Batch size bound for idempotent re-embed steps. */
  readonly batchSize: number;
}

// ---------------------------------------------------------------------------
// Outputs
// ---------------------------------------------------------------------------

export interface MemoryConsolidationOutput {
  readonly proposalId: string;
  readonly accepted: boolean;
  readonly canonicalMemoryId?: string;
}

export interface MemoryRetentionOutput {
  readonly swept: number;
  readonly deleted: number;
  readonly legalHoldProtected: number;
}

export interface MemoryLegalHoldOutput {
  readonly memoryId: string;
  readonly held: boolean;
  readonly releaseTime?: string;
}

export interface MemoryExportOutput {
  readonly exportRef: string;
  readonly records: number;
  readonly redacted: number;
}

export interface MemoryDeletionOutput {
  readonly memoryId: string;
  readonly deleted: boolean;
  readonly receiptId: string;
}

export interface MemoryReembedOutput {
  readonly reembedded: number;
  readonly failed: number;
  readonly targetEmbeddingVersion: string;
}

// ---------------------------------------------------------------------------
// Contracts
// ---------------------------------------------------------------------------

/**
 * Proposal-before-canonical consolidation workflow. Emits a proposal,
 * evaluates it, and promotes only accepted proposals to canonical
 * memory. Never writes canonical memory directly (SPEC-002 behavior 5).
 */
export const MemoryConsolidationWorkflow: MemoryWorkflowSpec<
  MemoryConsolidationInput,
  MemoryConsolidationOutput
> = {
  kind: "MEMORY_CONSOLIDATION",
  name: "nexus.memory-consolidation.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Bundles source observations into a MemoryProposal, evaluates it by policy, and promotes only accepted proposals to canonical memory. Models never write canonical memory directly (SPEC-002 behavior 5).",
  signals: ["APPROVAL", "CANCEL", "RESUME"],
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"],
  activities: [
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000101",
      "PROPOSE",
      "EXTERNAL_EFFECT",
      MEMORY_EFFECT_RETRY,
    ),
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000102",
      "EVALUATE_PROPOSAL",
      "VERIFY",
      MEMORY_VERIFY_RETRY,
    ),
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000103",
      "ACTIVATE_CANONICAL",
      "EXTERNAL_EFFECT",
      MEMORY_EFFECT_RETRY,
    ),
  ],
  policy: MEMORY_WORKFLOW_POLICY,
};

/** Deterministic retention sweep workflow (SPEC-002 requirement 8). */
export const MemoryRetentionWorkflow: MemoryWorkflowSpec<
  MemoryRetentionInput,
  MemoryRetentionOutput
> = {
  kind: "MEMORY_RETENTION",
  name: "nexus.memory-retention.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Deterministic retention sweep: expired records are deleted unless under legal hold. Legal hold preserves storage but never implies context relevance.",
  signals: ["CANCEL", "RESUME"],
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"],
  activities: [
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000111",
      "RETENTION_SWEEP",
      "EXTERNAL_EFFECT",
      MEMORY_EFFECT_RETRY,
    ),
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000112",
      "DELETE_RECORD",
      "EXTERNAL_EFFECT",
      MEMORY_EFFECT_RETRY,
      {
        activityId: parseActivityId("0193a1f2-0000-7000-8000-000000000113"),
        idempotencyKeyPrefix: "memory-retention-rollback",
        order: 1,
      },
    ),
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000113",
      "RETENTION_SWEEP",
      "VERIFY",
      MEMORY_VERIFY_RETRY,
    ),
  ],
  policy: MEMORY_WORKFLOW_POLICY,
};

/** Legal hold apply/release workflow (SPEC-002 requirement 8). */
export const MemoryLegalHoldWorkflow: MemoryWorkflowSpec<
  MemoryLegalHoldInput,
  MemoryLegalHoldOutput
> = {
  kind: "MEMORY_LEGAL_HOLD",
  name: "nexus.memory-legal-hold.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Applies or releases a legal hold on a memory record. Preserves storage against retention deletion; never surfaces the record into active context.",
  signals: ["APPROVAL", "CANCEL", "RESUME"],
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"],
  activities: [
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000121",
      "LEGAL_HOLD_APPLY",
      "EXTERNAL_EFFECT",
      MEMORY_EFFECT_RETRY,
    ),
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000122",
      "LEGAL_HOLD_RELEASE",
      "EXTERNAL_EFFECT",
      MEMORY_EFFECT_RETRY,
    ),
  ],
  policy: MEMORY_WORKFLOW_POLICY,
};

/** Audited tenant-scoped export workflow (SPEC-002 requirement 8). */
export const MemoryExportWorkflow: MemoryWorkflowSpec<
  MemoryExportInput,
  MemoryExportOutput
> = {
  kind: "MEMORY_EXPORT",
  name: "nexus.memory-export.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Audited, tenant-scoped snapshot export. The export artifact is produced by an idempotent activity; the workflow never touches filesystem or network directly.",
  signals: ["CANCEL", "RESUME"],
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"],
  activities: [
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000131",
      "EXPORT_SNAPSHOT",
      "EXTERNAL_EFFECT",
      MEMORY_EFFECT_RETRY,
    ),
  ],
  policy: MEMORY_WORKFLOW_POLICY,
};

/** Terminal audited deletion workflow (SPEC-002 requirement 8). */
export const MemoryDeletionWorkflow: MemoryWorkflowSpec<
  MemoryDeletionInput,
  MemoryDeletionOutput
> = {
  kind: "MEMORY_DELETION",
  name: "nexus.memory-deletion.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Terminal, audited deletion. Requires an explicit deletion digest; runs compensation when the delete is not verifiable.",
  signals: ["APPROVAL", "CANCEL", "RESUME"],
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"],
  activities: [
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000141",
      "DELETE_RECORD",
      "EXTERNAL_EFFECT",
      MEMORY_EFFECT_RETRY,
      {
        activityId: parseActivityId("0193a1f2-0000-7000-8000-000000000142"),
        idempotencyKeyPrefix: "memory-delete-rollback",
        order: 1,
      },
    ),
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000142",
      "DELETE_RECORD",
      "VERIFY",
      MEMORY_VERIFY_RETRY,
    ),
  ],
  policy: MEMORY_WORKFLOW_POLICY,
};

/** Re-embedding workflow after embedding schema change (SPEC-002 req 8). */
export const MemoryReembedWorkflow: MemoryWorkflowSpec<
  MemoryReembedInput,
  MemoryReembedOutput
> = {
  kind: "MEMORY_REEMBED",
  name: "nexus.memory-reembed.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Re-embeds memory records after an embedding schema change. Each re-embed step is an idempotent activity; failures are classified and retried within the bounded policy.",
  signals: ["CANCEL", "RESUME"],
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"],
  activities: [
    memoryActivity(
      "0193a1f2-0000-7000-8000-000000000151",
      "REEMBED",
      "EXTERNAL_EFFECT",
      MEMORY_EFFECT_RETRY,
    ),
  ],
  policy: MEMORY_WORKFLOW_POLICY,
};

/** Structural registry type for the memory workflow contracts. */
export type MemoryWorkflowRegistryEntry = Pick<
  MemoryWorkflowSpec<never, never>,
  | "kind"
  | "name"
  | "version"
  | "description"
  | "signals"
  | "queries"
  | "activities"
  | "policy"
>;

export const MEMORY_WORKFLOWS: readonly MemoryWorkflowRegistryEntry[] = [
  MemoryConsolidationWorkflow,
  MemoryRetentionWorkflow,
  MemoryLegalHoldWorkflow,
  MemoryExportWorkflow,
  MemoryDeletionWorkflow,
  MemoryReembedWorkflow,
];

/** Memory workflow kinds present in the registry. */
export const MEMORY_WORKFLOW_KINDS: readonly MemoryWorkflowKind[] = [
  "MEMORY_CONSOLIDATION",
  "MEMORY_RETENTION",
  "MEMORY_LEGAL_HOLD",
  "MEMORY_EXPORT",
  "MEMORY_DELETION",
  "MEMORY_REEMBED",
];

export type { PrincipalRef };
