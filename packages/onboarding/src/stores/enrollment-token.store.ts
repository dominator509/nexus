/**
 * EP-035 M3 EdgeEnrollment durable store (PostgreSQL 18.4).
 *
 * Enrollment credentials are one-time secrets. The store persists only
 * SHA-256 hashes of the secret and nonce (never the raw material) and
 * enforces the lifecycle at the durable boundary:
 *
 *   - ISSUED  : usable while within [issued_at, expires_at]
 *   - USED    : consumed exactly once (atomic claim)
 *   - REVOKED : never usable again
 *   - EXPIRED : never usable again
 *
 * Token possession alone is never device trust: the enrollment row
 * records state, and trust transitions belong to the contract layer.
 */

import { createHash, randomUUID } from "node:crypto";
import {
  EnrollmentCredential,
  type RedactedEnrollmentCredentialShape,
} from "@nexus/setup";
import { OnboardingDb, type QueryResultRow } from "../db.js";

export interface EnrollmentCredentialRow extends QueryResultRow {
  credential_id: string;
  kind: string;
  state: string;
  issued_at_unix_s: number;
  expires_at_unix_s: number;
  used_at_unix_s: number | null;
  revoked_at_unix_s: number | null;
  secret_hash: string;
  nonce_hash: string;
  correlation_id: string;
}

export function hashSecret(value: string): string {
  return createHash("sha256").update(`nexus-enrollment:${value}`).digest("hex");
}

export class EnrollmentTokenStore {
  constructor(private readonly db: OnboardingDb) {}

  /** Issue a fresh credential. The raw secret is never stored. */
  async issue(
    credential: EnrollmentCredential,
    correlationId?: string,
  ): Promise<RedactedEnrollmentCredentialShape> {
    await this.db.query(
      `INSERT INTO onboarding_enrollment_credential
         (credential_id, kind, state, issued_at_unix_s, expires_at_unix_s,
          secret_hash, nonce_hash, correlation_id)
       VALUES ($1, 'BOOTSTRAP_TOKEN', 'ISSUED', $2, $3, $4, $5, $6)`,
      [
        credential.credential_id,
        credential.issued_at_unix_s,
        credential.expires_at_unix_s,
        hashSecret(credential.secret),
        hashSecret(credential.nonce),
        correlationId ?? credential.credential_id,
      ],
      correlationId,
    );
    return credential.redacted();
  }

  /** Read a credential row by exact id (exact-target readback). */
  async read(
    credentialId: string,
    correlationId?: string,
  ): Promise<EnrollmentCredentialRow | undefined> {
    const res = await this.db.query<EnrollmentCredentialRow>(
      `SELECT credential_id, kind, state, issued_at_unix_s, expires_at_unix_s,
              used_at_unix_s, revoked_at_unix_s, secret_hash, nonce_hash,
              correlation_id
         FROM onboarding_enrollment_credential
        WHERE credential_id = $1`,
      [credentialId],
      correlationId,
    );
    return res.rows[0] as EnrollmentCredentialRow | undefined;
  }

  /**
   * Atomically claim a credential. Exactly one of:
   *   - a fresh ISSUED credential within its window -> state USED, true
   *   - anything else (used/revoked/expired/missing) -> false
   * The single UPDATE is the durable one-time boundary: two concurrent
   * claims race the row lock; exactly one flips ISSUED -> USED.
   */
  async claim(
    credentialId: string,
    nowUnixS: number,
    correlationId?: string,
  ): Promise<boolean> {
    const res = await this.db.query(
      `UPDATE onboarding_enrollment_credential
          SET state = 'USED', used_at_unix_s = $2
        WHERE credential_id = $1
          AND state = 'ISSUED'
          AND $2 BETWEEN issued_at_unix_s AND expires_at_unix_s
        RETURNING credential_id`,
      [credentialId, nowUnixS],
      correlationId,
    );
    return (res.rowCount ?? 0) === 1;
  }

  /** Verify a presented secret matches the stored hash (without claiming). */
  async verifySecret(
    credentialId: string,
    secret: string,
    correlationId?: string,
  ): Promise<boolean> {
    const row = await this.read(credentialId, correlationId);
    if (row === undefined) {
      return false;
    }
    return row.secret_hash === hashSecret(secret);
  }

  /** Revoke a credential permanently (never usable again). */
  async revoke(
    credentialId: string,
    nowUnixS: number,
    correlationId?: string,
  ): Promise<boolean> {
    const res = await this.db.query(
      `UPDATE onboarding_enrollment_credential
          SET state = 'REVOKED', revoked_at_unix_s = $2
        WHERE credential_id = $1
          AND state = 'ISSUED'
        RETURNING credential_id`,
      [credentialId, nowUnixS],
      correlationId,
    );
    return (res.rowCount ?? 0) === 1;
  }
}

export function newCredentialId(): string {
  return randomUUID();
}
