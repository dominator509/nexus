/**
 * EP-035 M3 DeploymentChoice durable store (PostgreSQL 18.4).
 *
 * A selected deployment profile is INTENT ONLY. The store persists the
 * selection and keeps SELECTED distinct from VERIFIED: verification
 * requires an explicit evidence record; a probe of the host never
 * mutates the user's selected intent.
 */

import {
  DeploymentIntentRecord,
  type DeploymentSelectionRequest,
  type DeploymentVerificationRequest,
  ErrorCode,
  Spec006Error,
} from "@nexus/setup";
import { OnboardingDb, type QueryResultRow } from "../db.js";

export interface DeploymentIntentRow extends QueryResultRow {
  intent_id: string;
  mode: string;
  release_channel: string;
  profile_json: unknown;
  verification_state: string;
  selected_at_unix_s: number;
  verified_at_unix_s: number | null;
  verification_evidence: unknown;
  correlation_id: string;
}

export class DeploymentIntentStore {
  constructor(private readonly db: OnboardingDb) {}

  /** Persist the user's selection as intent. Always SELECTED. */
  async recordSelection(
    intentId: string,
    request: DeploymentSelectionRequest,
    nowUnixS: number,
    correlationId?: string,
  ): Promise<DeploymentIntentRow> {
    const res = await this.db.query<DeploymentIntentRow>(
      `INSERT INTO onboarding_deployment_intent
         (intent_id, mode, release_channel, profile_json, verification_state,
          selected_at_unix_s, correlation_id)
       VALUES ($1, $2, $3, $4::jsonb, 'SELECTED', $5, $6)
       RETURNING intent_id, mode, release_channel, profile_json,
                 verification_state, selected_at_unix_s, verified_at_unix_s,
                 verification_evidence, correlation_id`,
      [
        intentId,
        request.profile.mode,
        request.profile.release_channel,
        JSON.stringify(request.profile.toJSON()),
        nowUnixS,
        correlationId ?? intentId,
      ],
      correlationId,
    );
    return res.rows[0] as DeploymentIntentRow;
  }

  /**
   * Record a verification evidence record. The state becomes VERIFIED
   * only when evidence exists (contract invariant: SELECTED != VERIFIED).
   */
  async recordVerification(
    intentId: string,
    request: DeploymentVerificationRequest,
    nowUnixS: number,
    correlationId?: string,
  ): Promise<DeploymentIntentRow> {
    const res = await this.db.query<DeploymentIntentRow>(
      `UPDATE onboarding_deployment_intent
          SET verification_state = 'VERIFIED',
              verified_at_unix_s = $2,
              verification_evidence = $3::jsonb
        WHERE intent_id = $1
          AND verification_state = 'SELECTED'
        RETURNING intent_id, mode, release_channel, profile_json,
                  verification_state, selected_at_unix_s, verified_at_unix_s,
                  verification_evidence, correlation_id`,
      [intentId, nowUnixS, JSON.stringify(request.evidence)],
      correlationId,
    );
    if ((res.rowCount ?? 0) !== 1) {
      throw new Spec006Error(
        ErrorCode.Conflict,
        "deployment intent not found or already verified",
        correlationId,
      );
    }
    return res.rows[0] as DeploymentIntentRow;
  }

  /** Read the exact intent row (exact-target readback). */
  async read(
    intentId: string,
    correlationId?: string,
  ): Promise<DeploymentIntentRow | undefined> {
    const res = await this.db.query<DeploymentIntentRow>(
      `SELECT intent_id, mode, release_channel, profile_json,
              verification_state, selected_at_unix_s, verified_at_unix_s,
              verification_evidence, correlation_id
         FROM onboarding_deployment_intent WHERE intent_id = $1`,
      [intentId],
      correlationId,
    );
    return res.rows[0] as DeploymentIntentRow | undefined;
  }
}

export function verifyDeploymentIntentRecord(
  row: DeploymentIntentRow,
): DeploymentIntentRecord {
  return DeploymentIntentRecord.parse({
    profile: row.profile_json,
    selected_at_unix_s: row.selected_at_unix_s,
    correlation_id: row.correlation_id,
    verification:
      row.verification_state === "VERIFIED"
        ? {
            state: "VERIFIED",
            evidence:
              row.verification_evidence === null
                ? undefined
                : row.verification_evidence,
          }
        : { state: "UNVERIFIED" },
  });
}
