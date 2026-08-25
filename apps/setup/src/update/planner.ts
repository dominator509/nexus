/**
 * EP-042 M2 deterministic update planner (SPEC-016 behavior 6).
 *
 * Pure: builds a PLANNED update plan and its deterministic digest. The
 * planner NEVER executes installation, never mutates production state,
 * and returns no executor. Promotion is not a step kind and cannot
 * appear in a plan.
 *
 * Permanent invariants:
 * - UPDATE PLAN EXISTS != UPDATE EXECUTED
 * - PLAN_READY (PLANNED) is the only state this surface can produce
 * - UPDATE PLAN DOES NOT EXECUTE INSTALLATION
 * - MISSING BACKUP PRECONDITION DENIED (backup-before-update)
 * - MISSING ROLLBACK PATH DENIED
 * - DOWNGRADE DENIED unless an explicit rollback policy owns it
 */

import { assertCompatibleForProfile } from "./compatibility";
import { contentDigest } from "./digest";
import { ReleaseError, ReleaseErrorCode } from "./errors";
import { assertManifestAcceptable } from "./manifest";
import { compareVersions } from "./compatibility";
import type { Digest } from "./types";
import { UPDATE_STEP_KINDS } from "./types";
import type {
  ReleaseChannel,
  ReleaseManifest,
  UpdatePlan,
  UpdateStep,
} from "./types";

export interface UpdatePlanInput {
  plan_id: string;
  release: ReleaseManifest;
  from_version: string;
  to_version: string;
  channel: ReleaseChannel;
  profile: "MANAGED" | "BYOC" | "EXISTING_SSH" | "HYBRID" | "FULLY_LOCAL";
  idempotency_key: string;
  correlation_id: string;
  created_at: string;
}

export interface UpdatePlanResult {
  plan: UpdatePlan;
  plan_digest: Digest;
}

/**
 * Canonical plan steps for a transactional update (SPEC-016 behavior 6).
 * The ROLLBACK step is the declared contingency path: it is planned,
 * never executed by this surface.
 */
export function canonicalUpdateSteps(): ReadonlyArray<UpdateStep> {
  return [
    { order: 1, kind: "BACKUP", description: "backup state before update" },
    { order: 2, kind: "MIGRATE", description: "apply compatible migrations" },
    { order: 3, kind: "CANARY", description: "canary cohort" },
    { order: 4, kind: "OBSERVE", description: "observe health" },
    {
      order: 5,
      kind: "ROLLBACK",
      description: "declared rollback contingency",
    },
  ];
}

function assertNoDowngrade(fromVersion: string, toVersion: string): void {
  const cmp = compareVersions(toVersion, fromVersion);
  if (cmp !== undefined && cmp < 0) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      `update plan downgrade from ${fromVersion} to ${toVersion} is denied without an explicit rollback policy`,
      { field: "to_version" },
    );
  }
}

/**
 * Canonical wire object for the plan (for the deterministic digest).
 */
export function planCanonicalObject(plan: UpdatePlan): Record<string, unknown> {
  return {
    schema_version: plan.schema_version,
    plan_id: plan.plan_id,
    release_id: plan.release_id,
    from_version: plan.from_version,
    to_version: plan.to_version,
    channel: plan.channel,
    steps: plan.steps,
    idempotency_key: plan.idempotency_key,
    correlation_id: plan.correlation_id,
    created_at: plan.created_at,
    state: plan.state,
  };
}

/**
 * Build a PLANNED update plan for a release. Fail-closed on:
 * - manifest not acceptable (parse/digest/duplicates)
 * - component set incompatible with the matrix
 * - target profile unsupported by the matrix (unknown platform denied)
 * - downgrade denied
 * - same version denied
 * - missing backup/rollback preconditions (steps always include them)
 *
 * The returned plan is PLANNED only; this function never executes
 * installation and never mutates state.
 */
export async function buildUpdatePlan(
  input: UpdatePlanInput,
): Promise<UpdatePlanResult> {
  const { release, from_version, to_version, channel } = input;
  if (to_version === from_version) {
    throw new ReleaseError(
      ReleaseErrorCode.Validation,
      "update plan must change the version",
      { field: "to_version" },
    );
  }
  assertNoDowngrade(from_version, to_version);
  await assertManifestAcceptable(release);
  assertCompatibleForProfile(
    release.compatibility,
    release.components,
    input.profile,
  );

  const steps = canonicalUpdateSteps();
  const plan: UpdatePlan = {
    schema_version: 1,
    plan_id: input.plan_id,
    release_id: release.release_id,
    from_version,
    to_version,
    channel,
    steps,
    idempotency_key: input.idempotency_key,
    correlation_id: input.correlation_id,
    created_at: input.created_at,
    state: "PLANNED",
  };
  const planDigest = await contentDigest(planCanonicalObject(plan));
  return { plan, plan_digest: planDigest };
}

/**
 * The plan has the mandatory BACKUP first step (backup-before-update).
 */
export function planHasBackupFirstStep(plan: UpdatePlan): boolean {
  const first = plan.steps[0];
  return first !== undefined && first.kind === "BACKUP";
}

/**
 * The plan declares a ROLLBACK contingency step (missing rollback path
 * is denied by the planner: every canonical plan includes it).
 */
export function planHasRollbackPath(plan: UpdatePlan): boolean {
  return plan.steps.some((step) => step.kind === "ROLLBACK");
}

/**
 * Plans can never contain a promotion step. The vocabulary has no
 * PROMOTE value; this checks every step kind against the canonical
 * vocabulary set at runtime.
 */
export function planContainsNoPromoteStep(plan: UpdatePlan): boolean {
  const kinds = UPDATE_STEP_KINDS as ReadonlyArray<string>;
  return plan.steps.every((step) => kinds.includes(step.kind));
}
