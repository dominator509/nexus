/**
 * EP-035 M3 RecoveryFlow durable store (PostgreSQL 18.4).
 *
 * Recovery is a real contract: checkpoints persist the mutation outcome
 * classification so no blind replay happens after an ambiguous external
 * mutation. The durable invariant is enforced in SQL:
 *
 *   mutation_state UNKNOWN  => retry_safe FALSE (never retry blind)
 *   mutation_state RECONCILED => decision made from readback
 *
 * The contract's decideRecovery() produces the canonical outcome; this
 * store persists it and records the reconciliation readback.
 */

import {
  decideRecovery,
  ErrorCode,
  type RecoveryDecision,
  type RecoveryEvidence,
  Spec006Error,
} from "@nexus/setup";
import { OnboardingDb, type QueryResultRow } from "../db.js";

export interface RecoveryCheckpointRow extends QueryResultRow {
  checkpoint_id: string;
  mutation_id: string;
  mutation_kind: string;
  mutation_state: string;
  failure_class: string;
  outcome: string;
  retry_safe: boolean;
  created_at_unix_s: number;
  reconciled_at_unix_s: number | null;
  detail: string;
  correlation_id: string;
}

export class RecoveryCheckpointStore {
  constructor(private readonly db: OnboardingDb) {}

  /**
   * Persist a recovery decision for an ambiguous or failed mutation.
   * The SQL CHECK enforces: UNKNOWN mutation_state can never be
   * retry-safe (no blind replay at the durable boundary).
   *
   * A retry-safe decision whose evidence proves no mutation occurred
   * persists the durable mutation_state as RECONCILED (the provider
   * readback already established the outcome) so the invariant holds.
   */
  async record(
    checkpointId: string,
    mutationId: string,
    mutationKind: string,
    evidence: RecoveryEvidence,
    decision: RecoveryDecision,
    nowUnixS: number,
    correlationId?: string,
  ): Promise<RecoveryCheckpointRow> {
    let durableState = decision.mutation_state;
    if (decision.retry_safe && durableState === "UNKNOWN") {
      // Retry is safe only when the evidence proves no mutation
      // occurred; that proof IS a reconciliation readback.
      if (!(evidence.mutation_known && evidence.mutation_occurred === false)) {
        throw new Spec006Error(
          ErrorCode.Policy,
          "refusing to persist a retry-safe decision for an unknown mutation",
          correlationId,
        );
      }
      durableState = "RECONCILED";
    }
    const res = await this.db.query<RecoveryCheckpointRow>(
      `INSERT INTO onboarding_recovery_checkpoint
         (checkpoint_id, mutation_id, mutation_kind, mutation_state,
          failure_class, outcome, retry_safe, created_at_unix_s, detail,
          correlation_id)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
       RETURNING checkpoint_id, mutation_id, mutation_kind, mutation_state,
                 failure_class, outcome, retry_safe, created_at_unix_s,
                 reconciled_at_unix_s, detail, correlation_id`,
      [
        checkpointId,
        mutationId,
        mutationKind,
        durableState,
        evidence.failure_class,
        decision.outcome,
        decision.retry_safe,
        nowUnixS,
        decision.detail,
        correlationId ?? checkpointId,
      ],
      correlationId,
    );
    return res.rows[0] as RecoveryCheckpointRow;
  }

  /**
   * Record the reconciliation readback: the provider state was read and
   * the mutation outcome is now known. Returns the updated row.
   */
  async reconcile(
    mutationId: string,
    mutationState: "UNKNOWN" | "RECONCILED",
    reconciledAtUnixS: number,
    correlationId?: string,
  ): Promise<RecoveryCheckpointRow> {
    const res = await this.db.query<RecoveryCheckpointRow>(
      `UPDATE onboarding_recovery_checkpoint
          SET mutation_state = $2,
              reconciled_at_unix_s = $3
        WHERE mutation_id = $1
        RETURNING checkpoint_id, mutation_id, mutation_kind, mutation_state,
                  failure_class, outcome, retry_safe, created_at_unix_s,
                  reconciled_at_unix_s, detail, correlation_id`,
      [mutationId, mutationState, reconciledAtUnixS],
      correlationId,
    );
    if ((res.rowCount ?? 0) !== 1) {
      throw new Spec006Error(
        ErrorCode.NotFound,
        "recovery checkpoint not found",
        correlationId,
      );
    }
    return res.rows[0] as RecoveryCheckpointRow;
  }

  /** Read the exact checkpoint row (exact-target readback). */
  async read(
    mutationId: string,
    correlationId?: string,
  ): Promise<RecoveryCheckpointRow | undefined> {
    const res = await this.db.query<RecoveryCheckpointRow>(
      `SELECT checkpoint_id, mutation_id, mutation_kind, mutation_state,
              failure_class, outcome, retry_safe, created_at_unix_s,
              reconciled_at_unix_s, detail, correlation_id
         FROM onboarding_recovery_checkpoint WHERE mutation_id = $1`,
      [mutationId],
      correlationId,
    );
    return res.rows[0] as RecoveryCheckpointRow | undefined;
  }

  /** Recompute the decision from durable state (pure contract reuse). */
  static decide(evidence: RecoveryEvidence): RecoveryDecision {
    return decideRecovery(evidence);
  }
}
