/**
 * EP-019 durable incident workflow contracts (SPEC-018; ADR-026).
 *
 * Nexus owns the self-healing engineering loop: incidents are detected
 * from real structured signals, correlated by canonical identifiers,
 * diagnosed with evidence, reproduced before consequential patching,
 * patched within an explicit scope, validated in an isolated
 * environment, security-checked, approved by a human at the configured
 * approval class, deployed in stages, verified post-deploy against the
 * original reproduction, and closed or rolled back. This module defines
 * the versioned workflow contracts over the incident plane:
 *
 * - IncidentLifecycleWorkflow: the canonical lifecycle driver.
 * - DiagnosisWorkflow: hypothesis -> evidence -> confidence escalation
 *   (a model can never self-certify root cause).
 * - ReproductionWorkflow: minimal reproduction before/after proof;
 *   UNREPRODUCIBLE is an explicit terminal.
 * - PatchProposalWorkflow: bounded-scope patch artifact generation.
 * - ReviewWorkflow: independent review bound to the exact patch digest.
 * - CanaryDeploymentWorkflow: staged deployment with health criteria
 *   and automatic rollback on regression.
 * - RollbackWorkflow: deterministic rollback to the known previous
 *   artifact/version.
 *
 * Hard invariants encoded here (same discipline as EP-006/EP-016/
 * EP-017):
 * 1. Workflow code is deterministic: no wall clock, network, database,
 *    filesystem, or random calls. All I/O happens through
 *    scheduleActivity() (SPEC-023 behavior 6).
 * 2. Every workflow is a versioned, named contract with a pinned signal
 *    and query surface. Breaking changes bump the name, never mutate a
 *    live version.
 * 3. Every activity is idempotent (idempotencyRequired) and carries a
 *    bounded, error-classified retry policy; PERMANENT is never
 *    retried.
 * 4. The lifecycle has NO collapsed "FIXED" state: only real observed
 *    post-deploy verification reaches CLOSED.
 * 5. Approval binds to the exact patch digest; approval of patch A can
 *    never authorize patch B.
 * 6. Rollback is bound to a known previous artifact/version and is
 *    never improvised from model-generated source.
 * 7. The self-healing system can never directly install its own
 *    generated skills; successful remediation only becomes a skill
 *    CANDIDATE through the EP-018 eval and approval flow.
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
import { incidentWorkflowKind } from "./vocabulary.js";
import type {
  CanaryOutcome,
  DiagnosisConfidence,
  IncidentOperationKind,
  IncidentReviewVerdict,
  IncidentWorkflowKind,
  RollbackOutcome,
} from "./vocabulary.js";

/** Activity idempotency keys are stable prefixes, never random. */
export const INCIDENT_ACTIVITY_IDEMPOTENCY_PREFIX = "incident-op";

const INCIDENT_EFFECT_RETRY: RetryPolicy = {
  ...DEFAULT_RETRY_POLICY,
  maxAttempts: 5,
  retryableErrorClasses: ["TRANSIENT", "RATE_LIMIT", "UNAVAILABLE", "TIMEOUT"],
};

const INCIDENT_VERIFY_RETRY: RetryPolicy = {
  ...DEFAULT_RETRY_POLICY,
  maxAttempts: 3,
};

/** A bounded incident activity with a stable idempotency key prefix. */
export interface IncidentActivityContract extends ActivityContract {
  readonly operationKind: IncidentOperationKind;
  readonly idempotencyKeyPrefix: string;
}

function incidentActivity(
  activityId: string,
  operationKind: IncidentOperationKind,
  kind: ActivityKind,
  retry: RetryPolicy,
  compensation?: CompensationStep,
): IncidentActivityContract {
  return {
    activityId: parseActivityId(activityId),
    kind,
    operationKind,
    idempotencyRequired: true,
    idempotencyKeyPrefix: `${INCIDENT_ACTIVITY_IDEMPOTENCY_PREFIX}:${operationKind.toLowerCase()}`,
    retry,
    timeoutMs: 10 * 60 * 1000,
    ...(compensation === undefined ? {} : { compensation }),
  };
}

const INCIDENT_WORKFLOW_POLICY: WorkflowPolicy = {
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
 * Incident workflow spec: the same structural surface as the EP-006
 * WorkflowSpec but pinned to the EP-019 incident workflow kinds and
 * incident activity contracts. EP-019-owned; the engine adapter
 * implements execute() against this surface.
 */
export interface IncidentWorkflowSpec<I = unknown, O = unknown> {
  readonly kind: IncidentWorkflowKind;
  /** Stable workflow name; breaking changes bump the name (versioning). */
  readonly name: string;
  /** Semver contract version; see src/versioning.ts. */
  readonly version: string;
  readonly description: string;
  readonly signals: readonly SignalType[];
  readonly queries: readonly QueryType[];
  readonly activities: readonly IncidentActivityContract[];
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

/** Common incident workflow input. */
export interface IncidentWorkflowInput extends WorkflowInput {
  /** Canonical incident id (SPEC-018 IncidentId). */
  readonly incidentId: string;
  /** Canonical correlation id (SPEC-003). */
  readonly correlationId: string;
  /** Tenant boundary (incidents are never merged across tenants). */
  readonly tenantId: string;
}

export interface IncidentLifecycleInput extends IncidentWorkflowInput {
  /** Canonical error class (not raw error text). */
  readonly errorClass: string;
  /** Affected component. */
  readonly component: string;
  /** Risk class R0..R4. */
  readonly risk: string;
}

export interface DiagnosisInput extends IncidentWorkflowInput {
  /** Model/agent-generated hypothesis. */
  readonly hypothesis: string;
}

export interface ReproductionInput extends IncidentWorkflowInput {
  /** Reproduction command/artifact reference. */
  readonly reproductionRef: string;
}

export interface PatchProposalInput extends IncidentWorkflowInput {
  /** Exact files changed (scope; unexpected expansion fails validation). */
  readonly filesChanged: readonly string[];
  /** The diff (patch artifact). */
  readonly diff: string;
  /** Rationale for the patch. */
  readonly rationale: string;
  /** Tests added/changed by the patch. */
  readonly testsChanged: readonly string[];
  /** Risk estimate R0..R4. */
  readonly risk: string;
  /** Canonical patch digest binding approvals and validation. */
  readonly patchDigest: string;
}

export interface ReviewInput extends IncidentWorkflowInput {
  /** Reviewer principal (independent of the proposer). */
  readonly reviewer: string;
  /** The exact patch digest under review. */
  readonly patchDigest: string;
}

export interface CanaryDeploymentInput extends IncidentWorkflowInput {
  /** Canonical patch digest being deployed. */
  readonly patchDigest: string;
  /** Stages in promotion order. */
  readonly stages: readonly string[];
  /** Health criteria observed during canary. */
  readonly healthCriteria: readonly string[];
}

export interface RollbackInput extends IncidentWorkflowInput {
  /** Known previous artifact/version to restore. */
  readonly previousArtifact: string;
  /** Deployment/version being rolled back. */
  readonly deployedVersion: string;
}

// ---------------------------------------------------------------------------
// Outputs
// ---------------------------------------------------------------------------

export interface IncidentLifecycleOutput {
  readonly incidentId: string;
  readonly finalState: string;
}

export interface DiagnosisOutput {
  readonly diagnosisId: string;
  readonly confidence: DiagnosisConfidence;
}

export interface ReproductionOutput {
  readonly reproduced: boolean;
  readonly beforeFailed: boolean;
  readonly afterPassed: boolean;
}

export interface PatchProposalOutput {
  readonly patchId: string;
  readonly patchDigest: string;
  readonly scopeOk: boolean;
}

export interface ReviewOutput {
  readonly verdict: IncidentReviewVerdict;
  readonly reviewedDigest: string;
}

export interface CanaryDeploymentOutput {
  readonly outcome: CanaryOutcome;
  readonly stage: string;
}

export interface RollbackOutput {
  readonly outcome: RollbackOutcome;
  readonly restoredArtifact: string;
}

// ---------------------------------------------------------------------------
// Contracts
// ---------------------------------------------------------------------------

/**
 * Incident lifecycle workflow (SPEC-018; ADR-026). Drives the canonical
 * lifecycle OBSERVE -> INCIDENT -> CORRELATE -> DIAGNOSE -> REPRODUCE ->
 * PATCH_PROPOSED -> SANDBOX_VALIDATION -> SECURITY_VALIDATION ->
 * APPROVAL -> STAGED_DEPLOYMENT -> POST_DEPLOY_VERIFICATION -> CLOSED.
 * There is no collapsed "FIXED" state: only real observed post-deploy
 * verification closes the incident.
 */
export const IncidentLifecycleWorkflow: IncidentWorkflowSpec<
  IncidentLifecycleInput,
  IncidentLifecycleOutput
> = {
  kind: "INCIDENT_LIFECYCLE",
  name: "nexus.incident-lifecycle.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Canonical self-healing lifecycle: observe -> correlate -> diagnose -> reproduce -> patch -> sandbox -> security -> approval -> staged deploy -> post-deploy verify -> closed; explicit terminal states, never collapsed.",
  signals: ["APPROVAL", "CANCEL", "RESUME"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000101",
      "OBSERVE_SIGNAL",
      "EXTERNAL_EFFECT",
      INCIDENT_EFFECT_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000102",
      "CORRELATE_INCIDENT",
      "EXTERNAL_EFFECT",
      INCIDENT_VERIFY_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000103",
      "CREATE_DIAGNOSIS",
      "EXTERNAL_EFFECT",
      INCIDENT_VERIFY_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000104",
      "RUN_REPRODUCTION",
      "VERIFY",
      INCIDENT_VERIFY_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000105",
      "GENERATE_PATCH",
      "EXTERNAL_EFFECT",
      INCIDENT_EFFECT_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000106",
      "VALIDATE_SANDBOX",
      "VERIFY",
      INCIDENT_VERIFY_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000107",
      "VALIDATE_SECURITY",
      "VERIFY",
      INCIDENT_VERIFY_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000108",
      "REQUEST_APPROVAL",
      "EXTERNAL_EFFECT",
      INCIDENT_EFFECT_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000109",
      "STAGE_DEPLOYMENT",
      "EXTERNAL_EFFECT",
      INCIDENT_EFFECT_RETRY,
      {
        activityId: parseActivityId("0193b1f2-0000-7000-8000-00000000010b"),
        idempotencyKeyPrefix: "incident-op:execute_rollback",
        order: 1,
      },
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-00000000010a",
      "VERIFY_POST_DEPLOY",
      "VERIFY",
      INCIDENT_VERIFY_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-00000000010b",
      "EXECUTE_ROLLBACK",
      "EXTERNAL_EFFECT",
      INCIDENT_EFFECT_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-00000000010c",
      "RECORD_MEMORY",
      "EXTERNAL_EFFECT",
      INCIDENT_VERIFY_RETRY,
    ),
  ],
  policy: INCIDENT_WORKFLOW_POLICY,
};

/**
 * Diagnosis workflow (SPEC-018; ADR-026). A model/agent may generate a
 * hypothesis; only reproducible evidence raises confidence to VALIDATED.
 * There is no "declare fixed" activity.
 */
export const DiagnosisWorkflow: IncidentWorkflowSpec<
  DiagnosisInput,
  DiagnosisOutput
> = {
  kind: "DIAGNOSIS",
  name: "nexus.incident-diagnosis.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Hypothesis -> evidence -> confidence escalation; a model can never self-certify root cause; VALIDATED requires reproducible evidence.",
  signals: ["CANCEL", "RESUME"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000103",
      "CREATE_DIAGNOSIS",
      "EXTERNAL_EFFECT",
      INCIDENT_VERIFY_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000104",
      "RUN_REPRODUCTION",
      "VERIFY",
      INCIDENT_VERIFY_RETRY,
    ),
  ],
  policy: INCIDENT_WORKFLOW_POLICY,
};

/**
 * Reproduction workflow (SPEC-018; ADR-026). Minimal reproduction
 * before/after proof; UNREPRODUCIBLE is an explicit terminal. Never
 * fabricate reproduction success.
 */
export const ReproductionWorkflow: IncidentWorkflowSpec<
  ReproductionInput,
  ReproductionOutput
> = {
  kind: "REPRODUCTION",
  name: "nexus.incident-reproduction.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Minimal reproduction before/after proof; UNREPRODUCIBLE is explicit; reproduction success is never fabricated.",
  signals: ["CANCEL"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000104",
      "RUN_REPRODUCTION",
      "VERIFY",
      INCIDENT_VERIFY_RETRY,
    ),
  ],
  policy: INCIDENT_WORKFLOW_POLICY,
};

/**
 * Patch proposal workflow (SPEC-018; ADR-026). Bounded-scope patch
 * artifact; unexpected scope expansion fails validation. The patch is a
 * proposal, never automatically applied production state.
 */
export const PatchProposalWorkflow: IncidentWorkflowSpec<
  PatchProposalInput,
  PatchProposalOutput
> = {
  kind: "PATCH_PROPOSAL",
  name: "nexus.incident-patch-proposal.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Bounded-scope patch artifact; unexpected scope expansion fails validation; patch is a proposal, not applied production state.",
  signals: ["CANCEL", "RESUME"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000105",
      "GENERATE_PATCH",
      "EXTERNAL_EFFECT",
      INCIDENT_EFFECT_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000106",
      "VALIDATE_SANDBOX",
      "VERIFY",
      INCIDENT_VERIFY_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000107",
      "VALIDATE_SECURITY",
      "VERIFY",
      INCIDENT_VERIFY_RETRY,
    ),
  ],
  policy: INCIDENT_WORKFLOW_POLICY,
};

/**
 * Independent review workflow (SPEC-018 behavior 4; ADR-026). The
 * reviewer is distinct from the proposer; the verdict binds to the exact
 * patch digest.
 */
export const ReviewWorkflow: IncidentWorkflowSpec<ReviewInput, ReviewOutput> = {
  kind: "REVIEW",
  name: "nexus.incident-review.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Independent review of root cause, diff, tests, security, compatibility, and rollback; verdict binds to the exact patch digest.",
  signals: ["APPROVAL", "CANCEL"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000108",
      "REQUEST_APPROVAL",
      "EXTERNAL_EFFECT",
      INCIDENT_EFFECT_RETRY,
    ),
  ],
  policy: INCIDENT_WORKFLOW_POLICY,
};

/**
 * Canary deployment workflow (SPEC-018; ADR-026). Staged deployment:
 * validated artifact -> canary -> health/readiness -> targeted
 * verification -> broader rollout. A canary regression automatically
 * rolls back and preserves evidence. Real production canary
 * certification is deferred to the deployment-owning node.
 */
export const CanaryDeploymentWorkflow: IncidentWorkflowSpec<
  CanaryDeploymentInput,
  CanaryDeploymentOutput
> = {
  kind: "CANARY_DEPLOYMENT",
  name: "nexus.incident-canary-deployment.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Staged deployment with health criteria; canary regression automatically rolls back and preserves evidence.",
  signals: ["CANCEL", "RESUME"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    incidentActivity(
      "0193b1f2-0000-7000-8000-000000000109",
      "STAGE_DEPLOYMENT",
      "EXTERNAL_EFFECT",
      INCIDENT_EFFECT_RETRY,
      {
        activityId: parseActivityId("0193b1f2-0000-7000-8000-00000000010b"),
        idempotencyKeyPrefix: "incident-op:execute_rollback",
        order: 1,
      },
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-00000000010a",
      "VERIFY_POST_DEPLOY",
      "VERIFY",
      INCIDENT_VERIFY_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-00000000010b",
      "EXECUTE_ROLLBACK",
      "EXTERNAL_EFFECT",
      INCIDENT_EFFECT_RETRY,
    ),
  ],
  policy: INCIDENT_WORKFLOW_POLICY,
};

/**
 * Rollback workflow (SPEC-018; ADR-026). Deterministic rollback to the
 * known previous artifact/version: version N healthy -> deploy N+1 ->
 * verification fails -> rollback -> version N restored -> health
 * restored. Never improvised from model-generated source.
 */
export const RollbackWorkflow: IncidentWorkflowSpec<
  RollbackInput,
  RollbackOutput
> = {
  kind: "ROLLBACK",
  name: "nexus.incident-rollback.v1",
  version: WORKFLOW_CONTRACT_VERSION,
  description:
    "Deterministic rollback to the known previous artifact/version; health restored and verified; never improvised from model-generated source.",
  signals: ["CANCEL"] as const,
  queries: ["WORKFLOW_STATUS", "ACTIVITY_STATE", "ACTION_RECEIPT"] as const,
  activities: [
    incidentActivity(
      "0193b1f2-0000-7000-8000-00000000010b",
      "EXECUTE_ROLLBACK",
      "EXTERNAL_EFFECT",
      INCIDENT_EFFECT_RETRY,
    ),
    incidentActivity(
      "0193b1f2-0000-7000-8000-00000000010a",
      "VERIFY_POST_DEPLOY",
      "VERIFY",
      INCIDENT_VERIFY_RETRY,
    ),
  ],
  policy: INCIDENT_WORKFLOW_POLICY,
};

/** Registry of every EP-019 incident workflow contract. */
export const INCIDENT_WORKFLOWS = [
  IncidentLifecycleWorkflow,
  DiagnosisWorkflow,
  ReproductionWorkflow,
  PatchProposalWorkflow,
  ReviewWorkflow,
  CanaryDeploymentWorkflow,
  RollbackWorkflow,
] as const;

export const INCIDENT_WORKFLOW_KINDS: readonly IncidentWorkflowKind[] =
  INCIDENT_WORKFLOWS.map((w) => w.kind);

/** Vocabulary guard: every registered kind is vocabulary locked. */
for (const workflow of INCIDENT_WORKFLOWS) {
  incidentWorkflowKind.parse(workflow.kind);
}
