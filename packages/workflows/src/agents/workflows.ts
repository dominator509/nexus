/**
 * EP-017 durable agent workflow contracts (SPEC-010; ADR-024).
 *
 * Nexus owns canonical objectives, task state, context, permissions,
 * budgets, artifacts, and results (SPEC-010 behavior 1). Agents request
 * capabilities rather than named agents; Nexus selects on quality,
 * cost, trust, availability, and historical success (behavior 2).
 * Direct agent-to-agent authority is forbidden; delegation passes
 * through Nexus policy and correlation (behavior 3). This module
 * defines the six versioned workflow contracts over the agent plane:
 *
 * - TaskAssignmentWorkflow: capability-based assignment. Candidate
 *   selection -> agent assignment -> session start; never a named
 *   peer.
 * - DelegationWorkflow: Nexus-recorded delegation lifecycle
 *   (PROPOSED -> ACCEPTED -> ACTIVE -> COMPLETED/REVOKED).
 * - ArtifactExchangeWorkflow: immutable artifact attach with lineage;
 *   a duplicate hash is a conflict, never a mutation.
 * - ReviewLoopWorkflow: bounded Codex-implement / Claude-review loop
 *   with APPROVE / REQUEST_CHANGES / REJECT verdicts and a hard
 *   iteration cap.
 * - CancellationWorkflow: cancel task + revoke active delegation with
 *   compensation; fails closed.
 * - BudgetEnforcementWorkflow: fail-closed budget consumption; POLICY
 *   on exhaustion, never a silent overrun.
 *
 * Hard invariants encoded here (same discipline as EP-006/EP-016):
 * 1. Workflow code is deterministic: no wall clock, network, database,
 *    filesystem, or random calls. All I/O happens through
 *    scheduleActivity() (SPEC-023 behavior 6).
 * 2. Every workflow is a versioned, named contract with a pinned signal
 *    and query surface. Breaking changes bump the name, never mutate a
 *    live version.
 * 3. Every activity is idempotent (idempotencyRequired) and carries a
 *    bounded, error-classified retry policy; PERMANENT is never
 *    retried.
 * 4. Timeout and cancel paths are explicit contracts: execution/run/task
 *    timeouts and cancellation semantics (CANCEL fails closed,
 *    COMPENSATE rolls back).
 */

import type { ActivityContract, CompensationStep } from "../activities.js";
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
import { agentWorkflowKind } from "./vocabulary.js";
import type {
  AgentOperationKind,
  AgentWorkflowKind,
  ArtifactDisposition,
  ReviewVerdict,
} from "./vocabulary.js";

/** Activity idempotency keys are stable prefixes, never random. */
export const AGENT_ACTIVITY_IDEMPOTENCY_PREFIX = "agent-op";

const AGENT_EFFECT_RETRY: RetryPolicy = {
  ...DEFAULT_RETRY_POLICY,
  maxAttempts: 5,
  retryableErrorClasses: ["TRANSIENT", "RATE_LIMIT", "UNAVAILABLE", "TIMEOUT"],
};

const AGENT_VERIFY_RETRY: RetryPolicy = {
  ...DEFAULT_RETRY_POLICY,
  maxAttempts: 3,
};

/** A bounded agent activity with a stable idempotency key prefix. */
export interface AgentActivityContract extends ActivityContract {
  readonly operationKind: AgentOperationKind;
  readonly idempotencyKeyPrefix: string;
}

function agentActivity(
  activityId: string,
  operationKind: AgentOperationKind,
  kind: ActivityKind,
  retry: RetryPolicy,
  compensation?: CompensationStep,
): AgentActivityContract {
  return {
    activityId: parseActivityId(activityId),
    kind,
    operationKind,
    idempotencyRequired: true,
    idempotencyKeyPrefix: `${AGENT_ACTIVITY_IDEMPOTENCY_PREFIX}:${operationKind.toLowerCase()}`,
    retry,
    timeoutMs: 10 * 60 * 1000,
    ...(compensation === undefined ? {} : { compensation }),
  };
}

const AGENT_WORKFLOW_POLICY: WorkflowPolicy = {
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
 * Agent workflow spec: the same structural surface as the EP-006
 * WorkflowSpec but pinned to the EP-017 agent workflow kinds and agent
 * activity contracts. EP-017-owned; the engine adapter implements
 * execute() against this surface, exactly as the EP-006 adapter does
 * for its kinds.
 */
export interface AgentWorkflowSpec<I = unknown, O = unknown> {
  readonly kind: AgentWorkflowKind;
  /** Stable workflow name; breaking changes bump the name (versioning). */
  readonly name: string;
  /** Semver contract version; see src/versioning.ts. */
  readonly version: string;
  readonly description: string;
  readonly signals: readonly SignalType[];
  readonly queries: readonly QueryType[];
  readonly activities: readonly AgentActivityContract[];
  readonly policy: WorkflowPolicy;
  readonly execute?: (
    ctx: import("../workflows.js").WorkflowContext,
    input: I,
  ) => Promise<
    { state: WorkflowState; outcome?: WorkflowOutcome } & {
      output?: O;
    }
  >;
}

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

/** Common agent workflow input. */
export interface AgentWorkflowInput extends WorkflowInput {
  /** Canonical task id (SPEC-001 TaskId). */
  readonly taskId: string;
  /** Canonical objective id (SPEC-001 ObjectiveId). */
  readonly objectiveId: string;
}

export interface TaskAssignmentInput extends AgentWorkflowInput {
  /** Requested capability (SPEC-010 behavior 2; ADR-024). */
  readonly capability: string;
  /** Least-privilege permission declarations. */
  readonly requiredPermissions: readonly string[];
}

export interface DelegationInput extends AgentWorkflowInput {
  /** Delegating principal (never another agent without Nexus policy). */
  readonly fromPrincipal: string;
  /** Selected agent card id (chosen by Nexus, never by a peer). */
  readonly toAgent: string;
}

export interface ArtifactExchangeInput extends AgentWorkflowInput {
  /** Immutable content hash (sha256 hex). */
  readonly contentHash: string;
  readonly name: string;
  readonly provenance: readonly string[];
}

export interface ReviewLoopInput extends AgentWorkflowInput {
  /** Review kind (for example code-review). */
  readonly reviewKind: string;
  /** Target artifact ids under review. */
  readonly targetArtifactIds: readonly string[];
  /** Hard iteration cap; REQUEST_CHANGES beyond this fails the task. */
  readonly maxIterations: number;
}

export interface CancellationInput extends AgentWorkflowInput {
  readonly reason: string;
}

export interface BudgetEnforcementInput extends AgentWorkflowInput {
  /** Budget class (ADR-024 AgentBudgetClass). */
  readonly budgetClass: string;
  readonly limit: number;
  readonly amount: number;
}

// ---------------------------------------------------------------------------
// Outputs
// ---------------------------------------------------------------------------

export interface TaskAssignmentOutput {
  readonly taskId: string;
  readonly assignedAgent: string;
  readonly capability: string;
  readonly started: boolean;
}

export interface DelegationOutput {
  readonly delegationId: string;
  readonly state: string;
}

export interface ArtifactExchangeOutput {
  readonly artifactId: string;
  readonly disposition: ArtifactDisposition;
}

export interface ReviewLoopOutput {
  readonly verdict: ReviewVerdict;
  readonly iterations: number;
  readonly approvedArtifactIds: readonly string[];
}

export interface CancellationOutput {
  readonly taskId: string;
  readonly cancelled: boolean;
  readonly delegationRevoked: boolean;
}

export interface BudgetEnforcementOutput {
  readonly taskId: string;
  readonly consumed: boolean;
  readonly remaining: number;
}

// ---------------------------------------------------------------------------
// Contracts
// ---------------------------------------------------------------------------

/**
 * Capability-based task assignment workflow (SPEC-010 behavior 2).
 * Selects candidates by capability, assigns the highest-ranked
 * eligible agent, and starts its session. Never selects a named peer.
 */
export const TaskAssignmentWorkflow: AgentWorkflowSpec<
  TaskAssignmentInput,
  TaskAssignmentOutput
> = {
  kind: "TASK_ASSIGNMENT",
  name: "nexus.agent-task-assignment.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Capability-based agent assignment: select candidates, assign the highest-ranked eligible agent, start the session. Never a named peer (SPEC-010 behavior 2).",
  signals: ["CANCEL"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    agentActivity(
      "0193a1f2-0000-7000-8000-000000000201",
      "SELECT_CANDIDATES",
      "EXTERNAL_EFFECT",
      AGENT_EFFECT_RETRY,
    ),
    agentActivity(
      "0193a1f2-0000-7000-8000-000000000202",
      "ASSIGN_AGENT",
      "EXTERNAL_EFFECT",
      AGENT_EFFECT_RETRY,
      {
        activityId: parseActivityId("0193a1f2-0000-7000-8000-000000000205"),
        idempotencyKeyPrefix: "agent-op:revoke_delegation",
        order: 1,
      },
    ),
    agentActivity(
      "0193a1f2-0000-7000-8000-000000000203",
      "START_SESSION",
      "EXTERNAL_EFFECT",
      AGENT_VERIFY_RETRY,
    ),
  ],
  policy: AGENT_WORKFLOW_POLICY,
};

/**
 * Nexus-recorded delegation workflow (SPEC-010 behavior 3). Direct
 * agent-to-agent authority is forbidden; every delegation is recorded
 * by Nexus and passes Nexus policy and correlation.
 */
export const DelegationWorkflow: AgentWorkflowSpec<
  DelegationInput,
  DelegationOutput
> = {
  kind: "DELEGATION",
  name: "nexus.agent-delegation.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Nexus-recorded delegation lifecycle PROPOSED -> ACCEPTED -> ACTIVE -> COMPLETED/REVOKED; direct agent-to-agent authority is forbidden.",
  signals: ["APPROVAL", "CANCEL"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    agentActivity(
      "0193a1f2-0000-7000-8000-000000000204",
      "RECORD_DELEGATION",
      "EXTERNAL_EFFECT",
      AGENT_EFFECT_RETRY,
    ),
    agentActivity(
      "0193a1f2-0000-7000-8000-000000000205",
      "REVOKE_DELEGATION",
      "EXTERNAL_EFFECT",
      AGENT_VERIFY_RETRY,
      {
        activityId: parseActivityId("0193a1f2-0000-7000-8000-000000000204"),
        idempotencyKeyPrefix: "agent-op:record_delegation",
        order: 1,
      },
    ),
  ],
  policy: AGENT_WORKFLOW_POLICY,
};

/**
 * Immutable artifact exchange workflow (SPEC-010; ADR-024). Attaches
 * an artifact by content hash with full provenance; a duplicate hash
 * is a conflict, never a mutation.
 */
export const ArtifactExchangeWorkflow: AgentWorkflowSpec<
  ArtifactExchangeInput,
  ArtifactExchangeOutput
> = {
  kind: "ARTIFACT_EXCHANGE",
  name: "nexus.agent-artifact-exchange.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Immutable artifact attach with lineage; duplicate content hash is a CONFLICT, never a mutation.",
  signals: ["CANCEL", "RESUME"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    agentActivity(
      "0193a1f2-0000-7000-8000-000000000206",
      "ATTACH_ARTIFACT",
      "EXTERNAL_EFFECT",
      AGENT_EFFECT_RETRY,
    ),
  ],
  policy: AGENT_WORKFLOW_POLICY,
};

/**
 * Bounded review loop workflow (SPEC-010 behavior 4). Codex-implement /
 * Claude-review with APPROVE / REQUEST_CHANGES / REJECT verdicts and a
 * hard iteration cap; exceeding the cap fails the task (never an
 * unbounded review loop).
 */
export const ReviewLoopWorkflow: AgentWorkflowSpec<
  ReviewLoopInput,
  ReviewLoopOutput
> = {
  kind: "REVIEW_LOOP",
  name: "nexus.agent-review-loop.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Bounded Codex-implement / Claude-review loop with APPROVE / REQUEST_CHANGES / REJECT and a hard iteration cap.",
  signals: ["APPROVAL", "CANCEL", "RESUME"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    agentActivity(
      "0193a1f2-0000-7000-8000-000000000207",
      "SUBMIT_REVIEW",
      "EXTERNAL_EFFECT",
      AGENT_VERIFY_RETRY,
    ),
  ],
  policy: AGENT_WORKFLOW_POLICY,
};

/**
 * Cancellation workflow (SPEC-023). Cancels the task, revokes the
 * active delegation, and runs compensation; fails closed on any
 * unverifiable step.
 */
export const CancellationWorkflow: AgentWorkflowSpec<
  CancellationInput,
  CancellationOutput
> = {
  kind: "CANCELLATION",
  name: "nexus.agent-cancellation.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Cancel task, revoke active delegation, run compensation; fails closed on unverifiable steps.",
  signals: ["CANCEL"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    agentActivity(
      "0193a1f2-0000-7000-8000-000000000208",
      "CANCEL_TASK",
      "EXTERNAL_EFFECT",
      AGENT_EFFECT_RETRY,
      {
        activityId: parseActivityId("0193a1f2-0000-7000-8000-000000000203"),
        idempotencyKeyPrefix: "agent-op:start_session",
        order: 1,
      },
    ),
    agentActivity(
      "0193a1f2-0000-7000-8000-000000000205",
      "REVOKE_DELEGATION",
      "EXTERNAL_EFFECT",
      AGENT_VERIFY_RETRY,
    ),
  ],
  policy: AGENT_WORKFLOW_POLICY,
};

/**
 * Budget enforcement workflow (SPEC-010; ADR-024). Consumes budget
 * fail-closed; POLICY on exhaustion, never a silent overrun.
 */
export const BudgetEnforcementWorkflow: AgentWorkflowSpec<
  BudgetEnforcementInput,
  BudgetEnforcementOutput
> = {
  kind: "BUDGET_ENFORCEMENT",
  name: "nexus.agent-budget-enforcement.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Fail-closed budget consumption; POLICY on exhaustion, never a silent overrun.",
  signals: ["CANCEL", "RESUME"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    agentActivity(
      "0193a1f2-0000-7000-8000-000000000209",
      "CONSUME_BUDGET",
      "EXTERNAL_EFFECT",
      AGENT_EFFECT_RETRY,
    ),
    agentActivity(
      "0193a1f2-0000-7000-8000-00000000020a",
      "PUBLISH_RESULT",
      "EXTERNAL_EFFECT",
      AGENT_VERIFY_RETRY,
    ),
  ],
  policy: AGENT_WORKFLOW_POLICY,
};

/** Registry of every EP-017 agent workflow contract. */
export const AGENT_WORKFLOWS = [
  TaskAssignmentWorkflow,
  DelegationWorkflow,
  ArtifactExchangeWorkflow,
  ReviewLoopWorkflow,
  CancellationWorkflow,
  BudgetEnforcementWorkflow,
] as const;

export const AGENT_WORKFLOW_KINDS: readonly AgentWorkflowKind[] =
  AGENT_WORKFLOWS.map((w) => w.kind);

/** Vocabulary guard: every registered kind is vocabulary locked. */
for (const workflow of AGENT_WORKFLOWS) {
  agentWorkflowKind.parse(workflow.kind);
}
