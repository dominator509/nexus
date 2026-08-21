/**
 * EP-035 M3 OwnerBootstrap durable store (PostgreSQL 18.4).
 *
 * Bridges the @nexus/setup OwnerBootstrap contract to the real durable
 * boundary. The onboarding_owner singleton unique index enforces
 * first-owner semantics at the persistence layer:
 *
 *   - no owner row + request            -> INSERT -> INITIALIZED
 *   - same idempotency_key replayed     -> ALREADY_INITIALIZED (same principal)
 *   - different key, owner exists       -> CONFLICT (unique index)
 *
 * Concurrency is real: two competing INSERTs race the unique partial
 * index; exactly one commits. This is not an in-process mutex.
 */

import { createHash, randomUUID } from "node:crypto";
import {
  ErrorCode,
  FirstOwnerKnown,
  type FirstOwnerResult,
  OwnerBootstrapRequest,
  OwnerBootstrapStateRecord,
  Spec006Error,
  resolveFirstOwnerRequest,
} from "@nexus/setup";
import { OnboardingDb, type QueryResultRow } from "../db.js";

export interface OwnerRow extends QueryResultRow {
  owner_id: string;
  idempotency_key: string;
  owner_email: string;
  state: string;
  correlation_id: string;
  created_at_unix_s: number;
  updated_at_unix_s: number;
}

export class OwnerBootstrapStore {
  constructor(private readonly db: OnboardingDb) {}

  /**
   * Initialize the first owner through the real durable boundary.
   * Returns the canonical FirstOwnerResult; never throws on conflict.
   */
  async initialize(
    request: OwnerBootstrapRequest,
    principalId: string,
    nowUnixS: number,
  ): Promise<FirstOwnerResult> {
    // 1. Read current owner (exact-target readback path also used by
    //    reconciliation): the durable state, not a client claim.
    const existing = await this.readOwner(request.correlation_id);

    if (existing !== undefined) {
      return resolveFirstOwnerRequest(
        new FirstOwnerKnown(existing.idempotency_key, existing.owner_id),
        request,
        principalId,
      );
    }

    // 2. No owner yet: race the singleton index. On unique violation the
    //    durable boundary already has an owner -> CONFLICT.
    try {
      const res = await this.db.query<OwnerRow>(
        `INSERT INTO onboarding_owner
           (owner_id, idempotency_key, owner_email, state, correlation_id,
            created_at_unix_s, updated_at_unix_s)
         VALUES ($1, $2, $3, 'OWNER_PRINCIPAL_CREATED', $4, $5, $5)
         RETURNING owner_id, idempotency_key, owner_email, state,
                   correlation_id, created_at_unix_s, updated_at_unix_s`,
        [
          principalId,
          request.idempotency_key,
          request.owner_email,
          request.correlation_id,
          nowUnixS,
        ],
        request.correlation_id,
      );
      const row = res.rows[0] as OwnerRow;
      return { kind: "INITIALIZED", principal_id: row.owner_id };
    } catch (err) {
      if (err instanceof Spec006Error && err.code === ErrorCode.Conflict) {
        // Competing bootstrap won the race; reconcile with durable state.
        const winner = await this.readOwner(request.correlation_id);
        if (winner !== undefined) {
          return resolveFirstOwnerRequest(
            new FirstOwnerKnown(winner.idempotency_key, winner.owner_id),
            request,
            principalId,
          );
        }
        return { kind: "CONFLICT" };
      }
      throw err;
    }
  }

  /**
   * Reconcile after an ambiguous bootstrap: read the exact durable
   * owner row. Returns undefined when no owner exists (safe to create).
   */
  async readOwner(correlationId?: string): Promise<OwnerRow | undefined> {
    const res = await this.db.query<OwnerRow>(
      `SELECT owner_id, idempotency_key, owner_email, state, correlation_id,
              created_at_unix_s, updated_at_unix_s
         FROM onboarding_owner LIMIT 1`,
      undefined,
      correlationId,
    );
    return res.rows[0] as OwnerRow | undefined;
  }

  /** Read the exact owner row by id (exact-target readback). */
  async readOwnerById(
    ownerId: string,
    correlationId?: string,
  ): Promise<OwnerRow | undefined> {
    const res = await this.db.query<OwnerRow>(
      `SELECT owner_id, idempotency_key, owner_email, state, correlation_id,
              created_at_unix_s, updated_at_unix_s
         FROM onboarding_owner WHERE owner_id = $1`,
      [ownerId],
      correlationId,
    );
    return res.rows[0] as OwnerRow | undefined;
  }

  /**
   * Record an owner bootstrap state transition (durable ladder record).
   * Uses the contract's transition validation.
   */
  async recordState(
    ownerId: string,
    record: OwnerBootstrapStateRecord,
    correlationId?: string,
  ): Promise<void> {
    if (record.principal_id !== undefined && record.principal_id !== ownerId) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "owner bootstrap state principal_id does not match owner row",
        correlationId,
      );
    }
    await this.db.query(
      `UPDATE onboarding_owner
          SET state = $1, updated_at_unix_s = $2
        WHERE owner_id = $3`,
      [record.state, record.updated_at_unix_s, ownerId],
      correlationId,
    );
  }
}

/**
 * Generate a deterministic principal id for a bootstrap request.
 * Derived from the request idempotency key (SHA-256 -> UUID bytes) so a
 * replayed request always reconciles to the same principal without
 * consulting any external identity provider in M3.
 */
export function derivePrincipalId(request: OwnerBootstrapRequest): string {
  const digest = createHash("sha256")
    .update(`nexus-owner-bootstrap:${request.idempotency_key}`)
    .digest();
  const bytes = digest.subarray(0, 16);
  bytes[6] = (bytes[6]! & 0x0f) | 0x50; // version 5
  bytes[8] = (bytes[8]! & 0x3f) | 0x80; // variant RFC 4122
  const hex = bytes.toString("hex");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
