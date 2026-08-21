/**
 * EP-035 M3 real PostgreSQL transport for the onboarding integration
 * layer. Wraps node-postgres and maps transport/constraint failures to
 * the canonical SPEC-006 vocabulary from @nexus/setup.
 *
 * Failure classes are never collapsed: connection refused -> UNAVAILABLE,
 * connect timeout -> TIMEOUT, unique violation -> CONFLICT, invalid
 * input -> VALIDATION, unknown -> INTERNAL.
 */

import { Pool, type PoolConfig, type QueryResult } from "pg";
import { ErrorCode, Spec006Error } from "@nexus/setup";

// pg returns BIGINT (int8, OID 20) as strings by default; onboarding
// timestamps are BIGINT columns, so parse them as numbers consistently.
import { types as pgTypes } from "pg";
pgTypes.setTypeParser(20, (val: string) => Number(val));

export interface OnboardingDbConfig {
  host: string;
  port: number;
  user: string;
  password: string;
  database: string;
  connectionTimeoutMillis?: number;
  statementTimeoutMillis?: number;
}

export class OnboardingDb {
  readonly pool: Pool;

  constructor(config: OnboardingDbConfig) {
    const cfg: PoolConfig = {
      host: config.host,
      port: config.port,
      user: config.user,
      password: config.password,
      database: config.database,
      connectionTimeoutMillis: config.connectionTimeoutMillis ?? 5000,
      statement_timeout: config.statementTimeoutMillis ?? 15000,
      max: 10,
    };
    this.pool = new Pool(cfg);
    // A Pool emits 'error' when a background client hits a fatal
    // connection problem (e.g. the container was removed mid-test or a
    // connection attempt fails outside an active query). Without a
    // listener this becomes an uncaught exception; with one, errors are
    // still surfaced per-query through the SPEC-006 mapping.
    this.pool.on("error", () => {
      // consumed; query() callers receive the mapped Spec006Error
    });
  }

  async close(): Promise<void> {
    await this.pool.end();
  }

  /** Map a thrown pg error to the canonical SPEC-006 class. */
  static mapError(err: unknown, correlationId?: string): Spec006Error {
    if (err instanceof Spec006Error) {
      return err;
    }
    const e = err as {
      code?: string;
      message?: string;
      name?: string;
    };
    if (e?.code === "23505") {
      return new Spec006Error(
        ErrorCode.Conflict,
        "duplicate value violates a unique constraint",
        correlationId,
      );
    }
    if (e?.code === "23503") {
      return new Spec006Error(
        ErrorCode.Conflict,
        "referenced value does not exist",
        correlationId,
      );
    }
    if (e?.code === "23514" || e?.code === "22P02" || e?.code === "22003") {
      return new Spec006Error(
        ErrorCode.Validation,
        "value violates a stored constraint",
        correlationId,
      );
    }
    if (e?.code === "23502") {
      return new Spec006Error(
        ErrorCode.Validation,
        "required value is missing",
        correlationId,
      );
    }
    if (e?.code === "ECONNREFUSED" || e?.code === "EHOSTUNREACH") {
      return new Spec006Error(
        ErrorCode.Unavailable,
        "database connection refused",
        correlationId,
      );
    }
    if (
      e?.code === "ETIMEDOUT" ||
      e?.code === "57014" ||
      e?.name === "TimeoutError" ||
      (typeof e?.message === "string" &&
        /timeout expired|connection timed out|connection timeout/i.test(
          e.message,
        ))
    ) {
      return new Spec006Error(
        ErrorCode.Timeout,
        "database operation timed out",
        correlationId,
      );
    }
    // A provider that dies mid-session surfaces as a reset/terminated
    // connection (container removal, crash, network drop), not a
    // refused connect. That is the same Unavailable class - the store
    // must never report Internal for a provider that is simply gone.
    // Timeout-flavored terminations ("...due to connection timeout")
    // are already classified as Timeout above.
    if (
      e?.code === "ECONNRESET" ||
      e?.code === "EPIPE" ||
      (typeof e?.message === "string" &&
        /terminat|connection closed|connection ended|socket hang up|server closed/i.test(
          e.message,
        ))
    ) {
      return new Spec006Error(
        ErrorCode.Unavailable,
        "database connection terminated",
        correlationId,
      );
    }
    if (e?.code === "28P01") {
      return new Spec006Error(
        ErrorCode.Authentication,
        "database authentication denied",
        correlationId,
      );
    }
    return new Spec006Error(
      ErrorCode.Internal,
      "database operation failed",
      correlationId,
    );
  }

  async query<T extends QueryResultRow = QueryResultRow>(
    text: string,
    params?: unknown[],
    correlationId?: string,
  ): Promise<QueryResult<T>> {
    try {
      return await this.pool.query<T>(text, params as never[]);
    } catch (err) {
      throw OnboardingDb.mapError(err, correlationId);
    }
  }

  /** Run the canonical onboarding DDL idempotently. */
  async migrate(correlationId?: string): Promise<void> {
    const sql = await import("./migration-sql.js");
    try {
      await this.pool.query(sql.MIGRATION_SQL);
    } catch (err) {
      throw OnboardingDb.mapError(err, correlationId);
    }
  }
}

export interface QueryResultRow {
  [column: string]: unknown;
}
