/**
 * EP-035 M3 IntegrationCard durable store (PostgreSQL 18.4).
 *
 * Real status is evidence-based: CONFIGURED != AUTHENTICATED !=
 * REACHABLE != HEALTHY. The store persists timestamps for each earned
 * level; a stored credential alone never implies HEALTHY. Capability
 * data is stored as supplied data (never name-derived).
 */

import {
  type IntegrationCardRequest,
  type IntegrationStatus,
  INTEGRATION_STATUSES,
  isValidIntegrationStatusTransition,
  ErrorCode,
  Spec006Error,
} from "@nexus/setup";
import { OnboardingDb, type QueryResultRow } from "../db.js";

export interface IntegrationStateRow extends QueryResultRow {
  integration_id: string;
  provider_name: string;
  status: string;
  configured_at_unix_s: number | null;
  authenticated_at_unix_s: number | null;
  reachable_at_unix_s: number | null;
  healthy_at_unix_s: number | null;
  capability_json: unknown;
  correlation_id: string;
  updated_at_unix_s: number;
}

const STATUS_COLUMN: Readonly<Record<IntegrationStatus, string>> = {
  UNCONFIGURED: "configured_at_unix_s",
  CONFIGURED: "configured_at_unix_s",
  AUTHENTICATED: "authenticated_at_unix_s",
  REACHABLE: "reachable_at_unix_s",
  HEALTHY: "healthy_at_unix_s",
  DEGRADED: "reachable_at_unix_s",
  ERROR: "configured_at_unix_s",
};

export class IntegrationStateStore {
  constructor(private readonly db: OnboardingDb) {}

  /**
   * Record a status transition with evidence timestamps. The transition
   * must be valid per the contract ladder (CONFIGURED -> HEALTHY alone is
   * rejected; REACHABLE requires a reachability verification, etc.).
   */
  async recordStatus(
    integrationId: string,
    request: IntegrationCardRequest,
    status: IntegrationStatus,
    nowUnixS: number,
    correlationId?: string,
  ): Promise<IntegrationStateRow> {
    const current = await this.read(integrationId, correlationId);
    if (current === undefined) {
      throw new Spec006Error(
        ErrorCode.NotFound,
        "integration card not found",
        correlationId,
      );
    }
    const from = current.status as IntegrationStatus;
    if (!isValidIntegrationStatusTransition(from, status)) {
      throw new Spec006Error(
        ErrorCode.Policy,
        `invalid integration status transition ${from} -> ${status}`,
        correlationId,
      );
    }
    if (!INTEGRATION_STATUSES.includes(status)) {
      throw new Spec006Error(
        ErrorCode.Vocabulary,
        `unknown integration status ${status}`,
        correlationId,
      );
    }

    const col = STATUS_COLUMN[status];
    const res = await this.db.query<IntegrationStateRow>(
      `UPDATE onboarding_integration_state
          SET status = $2,
              ${col} = $3,
              updated_at_unix_s = $4
        WHERE integration_id = $1
        RETURNING integration_id, provider_name, status,
                  configured_at_unix_s, authenticated_at_unix_s,
                  reachable_at_unix_s, healthy_at_unix_s,
                  capability_json, correlation_id, updated_at_unix_s`,
      [integrationId, status, nowUnixS, nowUnixS],
      correlationId,
    );
    return res.rows[0] as IntegrationStateRow;
  }

  /** Create the card row (UNCONFIGURED with provider identity). */
  async create(
    integrationId: string,
    providerName: string,
    nowUnixS: number,
    correlationId?: string,
  ): Promise<IntegrationStateRow> {
    const res = await this.db.query<IntegrationStateRow>(
      `INSERT INTO onboarding_integration_state
         (integration_id, provider_name, status, updated_at_unix_s,
          correlation_id)
       VALUES ($1, $2, 'UNCONFIGURED', $3, $4)
       RETURNING integration_id, provider_name, status,
                 configured_at_unix_s, authenticated_at_unix_s,
                 reachable_at_unix_s, healthy_at_unix_s,
                 capability_json, correlation_id, updated_at_unix_s`,
      [integrationId, providerName, nowUnixS, correlationId ?? integrationId],
      correlationId,
    );
    return res.rows[0] as IntegrationStateRow;
  }

  /** Replace the stored capability data (only when actually supplied). */
  async setCapabilities(
    integrationId: string,
    capabilities: unknown,
    correlationId?: string,
  ): Promise<IntegrationStateRow> {
    const res = await this.db.query<IntegrationStateRow>(
      `UPDATE onboarding_integration_state
          SET capability_json = $2::jsonb
        WHERE integration_id = $1
        RETURNING integration_id, provider_name, status,
                  configured_at_unix_s, authenticated_at_unix_s,
                  reachable_at_unix_s, healthy_at_unix_s,
                  capability_json, correlation_id, updated_at_unix_s`,
      [integrationId, JSON.stringify(capabilities)],
      correlationId,
    );
    if ((res.rowCount ?? 0) !== 1) {
      throw new Spec006Error(
        ErrorCode.NotFound,
        "integration card not found",
        correlationId,
      );
    }
    return res.rows[0] as IntegrationStateRow;
  }

  /** Read the exact integration row (exact-target readback). */
  async read(
    integrationId: string,
    correlationId?: string,
  ): Promise<IntegrationStateRow | undefined> {
    const res = await this.db.query<IntegrationStateRow>(
      `SELECT integration_id, provider_name, status,
              configured_at_unix_s, authenticated_at_unix_s,
              reachable_at_unix_s, healthy_at_unix_s,
              capability_json, correlation_id, updated_at_unix_s
         FROM onboarding_integration_state WHERE integration_id = $1`,
      [integrationId],
      correlationId,
    );
    return res.rows[0] as IntegrationStateRow | undefined;
  }
}
