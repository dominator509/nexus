/**
 * EP-042 M2 rollback precondition behavior (SPEC-016 behavior 6;
 * SPEC-024 restore).
 *
 * Pure and fail-closed. A rollback cannot be proven by a receipt alone:
 * it requires a declared rollback path in the plan, a target that is
 * compatible with the compatibility matrix, a well-formed receipt bound
 * to the correct version and install, and drill evidence. Absence of
 * drill evidence means "not proven", never "proven".
 *
 * Permanent invariants:
 * - ROLLBACK RECEIPT EXISTS != ROLLBACK PROVEN
 * - ROLLBACK PLAN MISSING -> DENIED
 * - ROLLBACK TARGET INCOMPATIBLE -> DENIED
 * - ROLLBACK RECEIPT MALFORMED -> DENIED
 * - ROLLBACK RECEIPT FROM WRONG VERSION/INSTALL ID -> DENIED
 * - ROLLBACK DRILL NOT RUN -> NOT PROVEN
 */

import { compareVersions, evaluateCompatibility } from "./compatibility";
import { ReleaseError, ReleaseErrorCode } from "./errors";
import { planHasRollbackPath } from "./planner";
import type {
  CompatibilityMatrix,
  RollbackReceipt,
  SignedComponent,
  UpdatePlan,
} from "./types";

export interface RollbackDrillEvidence {
  drill_id: string;
  install_id: string;
  from_version: string;
  to_version: string;
  verified_at: string;
  outcome: "VERIFIED" | "FAILED" | "NOT_RUN";
}

export interface RollbackVerdict {
  decision: "PROVEN" | "DENIED" | "NOT_PROVEN";
  reasons: ReadonlyArray<string>;
}

/**
 * Evaluate rollback preconditions for a plan, receipt, and drill
 * evidence.
 *
 * - plan without a ROLLBACK step -> DENIED (missing rollback path)
 * - receipt missing/malformed -> DENIED
 * - receipt from wrong plan/version -> DENIED
 * - rollback target incompatible with the matrix -> DENIED
 * - drill evidence missing or NOT_RUN -> NOT_PROVEN
 * - drill failed -> DENIED
 * - drill from wrong install/version -> DENIED
 * - all preconditions met and drill verified -> PROVEN
 */
export function evaluateRollbackPreconditions(
  plan: UpdatePlan,
  matrix: CompatibilityMatrix,
  components: ReadonlyArray<SignedComponent>,
  receipt: RollbackReceipt | undefined,
  drill: RollbackDrillEvidence | undefined,
): RollbackVerdict {
  if (!planHasRollbackPath(plan)) {
    return {
      decision: "DENIED",
      reasons: ["update plan has no rollback path"],
    };
  }
  if (receipt === undefined) {
    return {
      decision: "DENIED",
      reasons: ["rollback receipt missing"],
    };
  }
  const receiptReasons: Array<string> = [];
  if (receipt.receipt_id.trim() === "") {
    receiptReasons.push("rollback receipt has no receipt_id");
  }
  if (receipt.update_plan_ref !== plan.plan_id) {
    receiptReasons.push(
      `rollback receipt references plan ${receipt.update_plan_ref}, expected ${plan.plan_id}`,
    );
  }
  if (receipt.from_version !== plan.to_version) {
    receiptReasons.push(
      `rollback receipt from_version ${receipt.from_version} does not match plan target ${plan.to_version}`,
    );
  }
  if (receipt.to_version !== plan.from_version) {
    receiptReasons.push(
      `rollback receipt to_version ${receipt.to_version} does not match plan origin ${plan.from_version}`,
    );
  }
  if (
    receipt.backup_ref.backend.trim() === "" ||
    receipt.backup_ref.key.trim() === ""
  ) {
    receiptReasons.push("rollback receipt has no backup reference");
  }
  if (receiptReasons.length > 0) {
    return { decision: "DENIED", reasons: receiptReasons };
  }

  // Target compatibility: the rollback target (plan origin) must still
  // be compatible with the matrix.
  const targetVersion = plan.from_version;
  const cmp = compareVersions(targetVersion, plan.to_version);
  if (cmp !== undefined && cmp >= 0) {
    return {
      decision: "DENIED",
      reasons: [
        `rollback target ${targetVersion} is not older than ${plan.to_version}`,
      ],
    };
  }
  const compatibility = evaluateCompatibility(matrix, components);
  if (!compatibility.compatible) {
    return {
      decision: "DENIED",
      reasons: [
        `rollback target incompatible: ${compatibility.reasons.join("; ")}`,
      ],
    };
  }

  if (drill === undefined || drill.outcome === "NOT_RUN") {
    return {
      decision: "NOT_PROVEN",
      reasons: ["rollback drill has not been run"],
    };
  }
  if (drill.outcome === "FAILED") {
    return {
      decision: "DENIED",
      reasons: ["rollback drill failed"],
    };
  }
  if (drill.install_id !== receipt.backup_ref.key) {
    return {
      decision: "DENIED",
      reasons: [
        `rollback drill install_id ${drill.install_id} does not match receipt backup ${receipt.backup_ref.key}`,
      ],
    };
  }
  if (
    drill.from_version !== receipt.from_version ||
    drill.to_version !== receipt.to_version
  ) {
    return {
      decision: "DENIED",
      reasons: [
        `rollback drill versions ${drill.from_version}->${drill.to_version} do not match receipt ${receipt.from_version}->${receipt.to_version}`,
      ],
    };
  }
  return { decision: "PROVEN", reasons: [] };
}

/**
 * Fail-closed assertion for the promotion gate: rollback must be proven
 * before a canary may be considered ready. Throws on any denial or
 * not-proven state.
 */
export function assertRollbackProven(
  plan: UpdatePlan,
  matrix: CompatibilityMatrix,
  components: ReadonlyArray<SignedComponent>,
  receipt: RollbackReceipt | undefined,
  drill: RollbackDrillEvidence | undefined,
): void {
  const verdict = evaluateRollbackPreconditions(
    plan,
    matrix,
    components,
    receipt,
    drill,
  );
  if (verdict.decision !== "PROVEN") {
    throw new ReleaseError(
      ReleaseErrorCode.UnsafeRollback,
      `rollback precondition not proven: ${verdict.reasons.join("; ")}`,
      { field: "rollback" },
    );
  }
}
