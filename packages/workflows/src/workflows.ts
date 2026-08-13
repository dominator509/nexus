/**
 * Workflow definitions and the deterministic workflow context (SPEC-023
 * behaviors 5-6; ADR-010; EP-006 acceptance obligations).
 *
 * Hard invariants encoded here:
 *
 * 1. Workflow code is deterministic. The WorkflowContext exposes engine
 *    time only (`now()`), never the host wall clock, and no network,
 *    database, filesystem, or random surface. All I/O happens through
 *    scheduleActivity() and lives in activities (SPEC-023 behavior 6).
 *    The determinism guard (src/determinism.ts) audits every workflow
 *    source file for forbidden non-deterministic calls.
 *
 * 2. A workflow is a versioned, named contract (name + version + signal
 *    surface + query surface + activity surface + policy). The runtime
 *    engine (infra/temporal) implements execute() against this contract;
 *    the contract itself never imports an engine.
 */

import type { ActivityContract, PrincipalRef } from "./activities.js";
import type { ActionId, WorkflowId } from "./ids.js";
import type { WorkflowPolicy } from "./policies.js";
import type { WorkflowSignal } from "./signals.js";
import type { WorkflowQuery, WorkflowQueryResponse } from "./queries.js";
import type {
  ActivityKind,
  QueryType,
  SignalType,
  WorkflowKind,
  WorkflowOutcome,
  WorkflowState,
} from "./vocabulary.js";

/** Deterministic engine time. Never the host wall clock. */
export interface WorkflowTime {
  readonly millis: number;
}

/**
 * The only I/O and time surface a workflow may touch. Implemented by the
 * durable engine (Temporal) in infra/temporal; never re-implemented in
 * workflow code.
 */
export interface WorkflowContext {
  readonly workflowId: WorkflowId;
  readonly runId: string;
  readonly tenantId: string;
  readonly correlationId: string;
  readonly principal: PrincipalRef;
  /** Replay-stable start time from the engine. */
  readonly startedAt: WorkflowTime;
  /** Engine clock. Returns the same value on every replay step. */
  now(): WorkflowTime;
  /** Durable timer; replay-stable. */
  sleep(ms: number): Promise<void>;
  /** Wait for the next durable signal. */
  waitForSignal(): Promise<WorkflowSignal>;
  /** Number of signals of a type observed so far (idempotency queries). */
  signalCount(signalType: SignalType): Promise<number>;
  /** Read-only deterministic query against this workflow. */
  query<T extends WorkflowQueryResponse>(query: WorkflowQuery): Promise<T>;
  /**
   * Schedule an activity. Every side effect of the workflow must pass
   * through here; the engine deduplicates by the activity idempotency key.
   */
  scheduleActivity<T>(
    contract: ActivityContract,
    input: unknown,
  ): Promise<import("./activities.js").WorkflowActivityResult<T>>;
  /** Request cancellation with explicit cancel semantics from policy. */
  cancel(reason?: string): Promise<void>;
}

export interface WorkflowInput {
  readonly workflowId: WorkflowId;
  readonly tenantId: string;
  readonly correlationId: string;
  readonly principal: PrincipalRef;
}

export interface WorkflowResult {
  readonly state: WorkflowState;
  /** Terminal outcomes only; in-flight workflows have none yet. */
  readonly outcome?: WorkflowOutcome;
  /** One terminal receipt per mutation (SPEC-006 acceptance). */
  readonly receiptId?: string;
}

/**
 * A versioned workflow contract. `execute` is implemented by the engine
 * adapter in infra/temporal (M2); the contract pins the surface that any
 * implementation must honor so replay and versioning stay deterministic.
 */
export interface WorkflowSpec<I = unknown, O = unknown> {
  readonly kind: WorkflowKind;
  /** Stable workflow name; breaking changes bump the name (versioning). */
  readonly name: string;
  /** Semver contract version; see src/versioning.ts. */
  readonly version: string;
  readonly description: string;
  readonly signals: readonly SignalType[];
  readonly queries: readonly QueryType[];
  readonly activities: readonly ActivityContract[];
  readonly policy: WorkflowPolicy;
  readonly execute?: (
    ctx: WorkflowContext,
    input: I,
  ) => Promise<WorkflowResult & { output?: O }>;
}

export interface ObjectiveInput extends WorkflowInput {
  readonly objectiveId: import("./ids.js").ObjectiveId;
  readonly title: string;
  readonly milestones: readonly {
    readonly milestoneId: import("./ids.js").ActivityId;
    readonly title: string;
    readonly actionId: ActionId;
    readonly actionDigest: import("./ids.js").ActionDigest;
  }[];
}

export interface ApprovalInput extends WorkflowInput {
  readonly actionId: ActionId;
  readonly actionDigest: import("./ids.js").ActionDigest;
  /** Approval class requirement (SPEC-005 behavior 4). */
  readonly requiredAuthenticationStrength: import("./vocabulary.js").AuthenticationStrength;
  /**
   * Optional per-workflow approval deadline in milliseconds. When set, it
   * overrides the contract default approvalTimeoutMs so each workflow can
   * carry an explicit timeout (SPEC-023 error-state TIMEOUT; EP-006
   * invariant: timeout paths are explicit and testable). Omit to use the
   * contract policy default.
   */
  readonly approvalTimeoutMs?: number;
}

export interface ConnectorCertificationInput extends WorkflowInput {
  readonly connectorId: import("./ids.js").ActivityId;
  readonly provider: string;
  /**
   * Certification steps, each bound to the exact action digest the
   * approval will be checked against (ADR-010 digest-binding invariant).
   */
  readonly steps: readonly {
    readonly stepId: import("./ids.js").ActivityId;
    readonly title: string;
    readonly actionId: ActionId;
    readonly actionDigest: import("./ids.js").ActionDigest;
  }[];
}

export interface IncidentRemediationInput extends WorkflowInput {
  readonly incidentId: import("./ids.js").ActivityId;
  readonly severity: string;
  readonly diagnosis: string;
  readonly remediationPlan: readonly {
    readonly stepId: import("./ids.js").ActivityId;
    readonly title: string;
    readonly actionId: ActionId;
    readonly actionDigest: import("./ids.js").ActionDigest;
  }[];
}

export interface DeploymentInput extends WorkflowInput {
  readonly releaseId: import("./ids.js").ActivityId;
  readonly stages: readonly {
    readonly stageId: import("./ids.js").ActivityId;
    readonly name: string;
    readonly actionId: ActionId;
    readonly actionDigest: import("./ids.js").ActionDigest;
  }[];
  readonly canary: boolean;
}

export interface ObjectiveOutput {
  readonly objectiveId: import("./ids.js").ObjectiveId;
  readonly completedMilestones: readonly string[];
}

export interface ApprovalOutput {
  readonly actionId: ActionId;
  readonly actionDigest: import("./ids.js").ActionDigest;
  readonly decision: "APPROVE" | "REJECT";
}

export interface ConnectorCertificationOutput {
  readonly connectorId: import("./ids.js").ActivityId;
  readonly certified: boolean;
  readonly evidenceRef: string;
}

export interface IncidentRemediationOutput {
  readonly incidentId: import("./ids.js").ActivityId;
  readonly remediated: boolean;
  readonly verificationRef: string;
}

export interface DeploymentOutput {
  readonly releaseId: import("./ids.js").ActivityId;
  readonly deployed: boolean;
  readonly rollbackRequired: boolean;
}

// ---------------------------------------------------------------------------
// The five workflow contracts (EP-006 node contract public interfaces).
// Each declares its versioned name, signal/query surface, activity surface,
// and policy. execute() implementations live in infra/temporal (M2); the
// contract pins the surface so replay and versioning stay deterministic.
// ---------------------------------------------------------------------------

import { parseActivityId } from "./ids.js";
import { DEFAULT_RETRY_POLICY } from "./policies.js";
import type { RetryPolicy } from "./policies.js";
import { WORKFLOW_CONTRACT_VERSION } from "./versioning.js";

const EFFECT_RETRY: RetryPolicy = {
  ...DEFAULT_RETRY_POLICY,
  maxAttempts: 5,
  retryableErrorClasses: ["TRANSIENT", "RATE_LIMIT", "UNAVAILABLE", "TIMEOUT"],
};

const VERIFY_RETRY: RetryPolicy = {
  ...DEFAULT_RETRY_POLICY,
  maxAttempts: 3,
};

const activity = (
  activityId: string,
  kind: ActivityKind,
  retry: RetryPolicy,
  compensation?: import("./activities.js").CompensationStep,
): ActivityContract => ({
  activityId: parseActivityId(activityId),
  kind,
  idempotencyRequired: true,
  retry,
  timeoutMs: 10 * 60 * 1000,
  ...(compensation === undefined ? {} : { compensation }),
});

const APPROVAL_POLICY: WorkflowPolicy = {
  timeouts: {
    executionTimeoutMs: 30 * 24 * 60 * 60 * 1000,
    runTimeoutMs: 30 * 24 * 60 * 60 * 1000,
    taskTimeoutMs: 10 * 60 * 1000,
    approvalTimeoutMs: 5 * 24 * 60 * 60 * 1000,
  },
  cancelAction: "COMPENSATE",
  defaultActivityRetry: DEFAULT_RETRY_POLICY,
};

export const ObjectiveWorkflow: WorkflowSpec<ObjectiveInput, ObjectiveOutput> =
  {
    kind: "OBJECTIVE",
    name: "nexus.objective.v1",
    version: WORKFLOW_CONTRACT_VERSION,
    description:
      "Long-running objective with milestone approvals; each milestone effect runs as an idempotent EXTERNAL_EFFECT activity, is verified, and is compensated on failure.",
    signals: ["APPROVAL", "CANCEL", "RESUME"],
    queries: [
      "WORKFLOW_STATUS",
      "PENDING_APPROVAL",
      "ACTIVITY_STATE",
      "ACTION_RECEIPT",
    ],
    activities: [
      activity(
        "0193a1f2-0000-7000-8000-000000000011",
        "EXTERNAL_EFFECT",
        EFFECT_RETRY,
        {
          activityId: parseActivityId("0193a1f2-0000-7000-8000-000000000013"),
          idempotencyKeyPrefix: "objective-milestone-rollback",
          order: 1,
        },
      ),
      activity("0193a1f2-0000-7000-8000-000000000012", "VERIFY", VERIFY_RETRY),
      activity(
        "0193a1f2-0000-7000-8000-000000000013",
        "COMPENSATE",
        VERIFY_RETRY,
      ),
    ],
    policy: APPROVAL_POLICY,
  };

export const ApprovalWorkflow: WorkflowSpec<ApprovalInput, ApprovalOutput> = {
  kind: "APPROVAL",
  name: "nexus.approval.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Durable human approval gate. Waits for an ApprovalSignal bound to the exact action digest and required authentication strength; expires on approvalTimeoutMs; cancellation triggers compensation.",
  signals: ["APPROVAL", "CANCEL", "RESUME"],
  queries: ["WORKFLOW_STATUS", "PENDING_APPROVAL", "ACTION_RECEIPT"],
  activities: [
    activity("0193a1f2-0000-7000-8000-000000000021", "VERIFY", VERIFY_RETRY),
    activity(
      "0193a1f2-0000-7000-8000-000000000022",
      "COMPENSATE",
      VERIFY_RETRY,
    ),
  ],
  policy: APPROVAL_POLICY,
};

export const ConnectorCertificationWorkflow: WorkflowSpec<
  ConnectorCertificationInput,
  ConnectorCertificationOutput
> = {
  kind: "CONNECTOR_CERTIFICATION",
  name: "nexus.connector-certification.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Certifies a connector against a real provider: runs certification EXTERNAL_EFFECT activities, verifies each result, and records evidence; failure compensates.",
  signals: ["APPROVAL", "CANCEL", "RESUME"],
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"],
  activities: [
    activity(
      "0193a1f2-0000-7000-8000-000000000031",
      "EXTERNAL_EFFECT",
      EFFECT_RETRY,
      {
        activityId: parseActivityId("0193a1f2-0000-7000-8000-000000000033"),
        idempotencyKeyPrefix: "cert-run-rollback",
        order: 1,
      },
    ),
    activity("0193a1f2-0000-7000-8000-000000000032", "VERIFY", VERIFY_RETRY),
    activity(
      "0193a1f2-0000-7000-8000-000000000033",
      "COMPENSATE",
      VERIFY_RETRY,
    ),
  ],
  policy: APPROVAL_POLICY,
};

export const IncidentRemediationWorkflow: WorkflowSpec<
  IncidentRemediationInput,
  IncidentRemediationOutput
> = {
  kind: "INCIDENT_REMEDIATION",
  name: "nexus.incident-remediation.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Remediation loop: HITL approval per remediation step, idempotent remediation effect, verification, and compensation on failure (SPEC-018 discipline).",
  signals: ["APPROVAL", "CANCEL", "RESUME"],
  queries: [
    "WORKFLOW_STATUS",
    "PENDING_APPROVAL",
    "ACTIVITY_STATE",
    "ACTION_RECEIPT",
  ],
  activities: [
    activity(
      "0193a1f2-0000-7000-8000-000000000041",
      "EXTERNAL_EFFECT",
      EFFECT_RETRY,
      {
        activityId: parseActivityId("0193a1f2-0000-7000-8000-000000000043"),
        idempotencyKeyPrefix: "remediation-rollback",
        order: 1,
      },
    ),
    activity("0193a1f2-0000-7000-8000-000000000042", "VERIFY", VERIFY_RETRY),
    activity(
      "0193a1f2-0000-7000-8000-000000000043",
      "COMPENSATE",
      VERIFY_RETRY,
    ),
  ],
  policy: APPROVAL_POLICY,
};

export const DeploymentWorkflow: WorkflowSpec<
  DeploymentInput,
  DeploymentOutput
> = {
  kind: "DEPLOYMENT",
  name: "nexus.deployment.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Staged rollout with canary: each stage requires approval, runs the deploy EXTERNAL_EFFECT, verifies, and rolls back (compensates) on verification failure.",
  signals: ["APPROVAL", "CANCEL", "RESUME"],
  queries: [
    "WORKFLOW_STATUS",
    "PENDING_APPROVAL",
    "ACTIVITY_STATE",
    "ACTION_RECEIPT",
  ],
  activities: [
    activity(
      "0193a1f2-0000-7000-8000-000000000051",
      "EXTERNAL_EFFECT",
      EFFECT_RETRY,
      {
        activityId: parseActivityId("0193a1f2-0000-7000-8000-000000000053"),
        idempotencyKeyPrefix: "deploy-stage-rollback",
        order: 1,
      },
    ),
    activity("0193a1f2-0000-7000-8000-000000000052", "VERIFY", VERIFY_RETRY),
    activity(
      "0193a1f2-0000-7000-8000-000000000053",
      "COMPENSATE",
      VERIFY_RETRY,
    ),
  ],
  policy: APPROVAL_POLICY,
};

/** Structural registry type: all five workflow contracts share this surface. */
export type WorkflowRegistryEntry = Pick<
  WorkflowSpec<never, never>,
  | "kind"
  | "name"
  | "version"
  | "description"
  | "signals"
  | "queries"
  | "activities"
  | "policy"
>;

export const WORKFLOWS: readonly WorkflowRegistryEntry[] = [
  ObjectiveWorkflow,
  ApprovalWorkflow,
  ConnectorCertificationWorkflow,
  IncidentRemediationWorkflow,
  DeploymentWorkflow,
];

export type { ActivityKind, WorkflowOutcome, WorkflowState };
