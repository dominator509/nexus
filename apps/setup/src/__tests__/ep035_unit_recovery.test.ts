/**
 * EP-035 M1 RecoveryFlow no-blind-replay tests.
 *
 * UNKNOWN WHETHER EXTERNAL MUTATION OCCURRED -> RECONCILE FIRST ->
 * RETRY ONLY IF SAFE. RecoveryKit binds the canonical recovery-kit
 * schema.
 */

import { describe, expect, it } from "vitest";
import {
  RecoveryEvidence,
  RecoveryKit,
  decideRecovery,
} from "../contracts/recovery";
import { ErrorCode, Spec006Error } from "../contracts/errors";

const KIT_ID = "00000000-0000-4000-8000-000000000001";
const PRINCIPAL = "00000000-0000-4000-8000-000000000002";
const TENANT = "00000000-0000-4000-8000-000000000003";

function evidence(
  failure_class: string,
  mutation_known: boolean,
  extra: Record<string, unknown> = {},
): RecoveryEvidence {
  return RecoveryEvidence.parse({
    failure_class,
    mutation_known,
    ...extra,
  });
}

describe("ep035_unit_recovery", () => {
  it("ambiguous mutation forces reconcile, never blind retry", () => {
    const decision = decideRecovery(evidence("AMBIGUOUS", false));
    expect(decision.outcome).toBe("RECONCILE");
    expect(decision.retry_safe).toBe(false);
  });

  it("an ambiguous mutation is retry-safe only with an explicit negative observation (AUD-045)", () => {
    const decision = decideRecovery(
      evidence("AMBIGUOUS", true, {
        mutation_occurred: true,
        mutation_state: "RECONCILED",
      }),
    );
    expect(decision.outcome).toBe("RECONCILE");
    expect(decision.retry_safe).toBe(false);
  });

  it("ambiguous + reconciled with KNOWN no-mutation is retry-safe (AUD-045)", () => {
    const decision = decideRecovery(
      evidence("AMBIGUOUS", true, {
        mutation_occurred: false,
        mutation_state: "RECONCILED",
      }),
    );
    expect(decision.outcome).toBe("RETRYABLE");
    expect(decision.retry_safe).toBe(true);
  });

  it("ambiguous + reconciled WITHOUT a known mutation is NOT retry-safe (AUD-045)", () => {
    const decision = decideRecovery(
      evidence("AMBIGUOUS", false, {
        mutation_state: "RECONCILED",
      }),
    );
    expect(decision.outcome).toBe("RECONCILE");
    expect(decision.retry_safe).toBe(false);
  });

  it("timeout with unknown mutation is never blindly retried", () => {
    const decision = decideRecovery(evidence("TIMEOUT", false));
    expect(decision.outcome).toBe("RECONCILE");
    expect(decision.retry_safe).toBe(false);
  });

  it("timeout with known no-mutation is retryable and safe", () => {
    const decision = decideRecovery(
      evidence("TIMEOUT", true, { mutation_occurred: false }),
    );
    expect(decision.outcome).toBe("RETRYABLE");
    expect(decision.retry_safe).toBe(true);
  });

  it("validation failures are non-retryable until input is corrected", () => {
    const decision = decideRecovery(evidence("VALIDATION", true));
    expect(decision.outcome).toBe("NON_RETRYABLE");
    expect(decision.retry_safe).toBe(false);
  });

  it("authorization failures require reauthentication", () => {
    const decision = decideRecovery(evidence("AUTHORIZATION", true));
    expect(decision.outcome).toBe("REAUTHENTICATE");
    expect(decision.retry_safe).toBe(false);
  });

  it("conflicts resume from the checkpoint instead of replaying", () => {
    const decision = decideRecovery(evidence("CONFLICT", true));
    expect(decision.outcome).toBe("RESUME_CHECKPOINT");
    expect(decision.retry_safe).toBe(false);
  });

  it("internal failures require manual intervention", () => {
    const decision = decideRecovery(evidence("INTERNAL", true));
    expect(decision.outcome).toBe("MANUAL_INTERVENTION");
    expect(decision.retry_safe).toBe(false);
  });

  it("recovery kit binds the canonical schema with deny-unknown", () => {
    const kit = RecoveryKit.parse({
      kit_id: KIT_ID,
      principal_id: PRINCIPAL,
      tenant_id: TENANT,
      material_kind: "RECOVERY_CODES",
      created_at_unix_s: 1000,
      expires_at_unix_s: 2000,
      correlation: "00000000-0000-4000-8000-000000000004",
    });
    expect(kit.material_kind).toBe("RECOVERY_CODES");
    expect(kit.isExpired(3000)).toBe(true);
    expect(kit.isExpired(1500)).toBe(false);
    expect(() =>
      RecoveryKit.parse({
        ...kit.toJSON(),
        forged: true,
      }),
    ).toThrowError(Spec006Error);
    expect(() =>
      RecoveryKit.parse({
        ...kit.toJSON(),
        material_kind: "MADE_UP",
      }),
    ).toThrowError(Spec006Error);
    expect(() =>
      RecoveryKit.parse({
        ...kit.toJSON(),
        expires_at_unix_s: 500,
      }),
    ).toThrowError(Spec006Error);
  });

  it("rejects invalid failure classes and non-boolean mutation_known", () => {
    expect(() => evidence("MADE_UP", true)).toThrowError(Spec006Error);
    expect(() =>
      RecoveryEvidence.parse({
        failure_class: "TIMEOUT",
        mutation_known: "yes",
      }),
    ).toThrowError(Spec006Error);
  });
});
