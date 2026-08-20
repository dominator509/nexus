/**
 * EP-033 M1 authenticated session model.
 *
 * Binds the dashboard session to the canonical auth-session schema
 * (schemas/auth/auth-session.schema.json) and the business/workspace
 * context to the canonical Hydra business-context schema
 * (schemas/hydra/business-context.schema.json). Field names and enums
 * are the canonical snake_case wire names verbatim; parity is enforced
 * by ep033_unit_schema_parity tests reading the schema files.
 *
 * Invariants (EP-033 directive F/G/N):
 * - principal/tenant/business context is bound explicitly by typed ids,
 *   never inferred from a screen name or cached label;
 * - switching business/workspace invalidates the previous context's
 *   projections: stale data is not actionable after the switch;
 * - an expired or revoked session fails closed: consequential mutation
 *   is refused and is never queued for blind replay.
 */

import {
  assertBool,
  assertEnum,
  assertInt,
  assertObject,
  assertString,
  assertUuid,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const GRANT_FLOWS = [
  "AUTHORIZATION_CODE",
  "REFRESH_TOKEN",
  "CLIENT_CREDENTIALS",
] as const;
export type GrantFlow = (typeof GRANT_FLOWS)[number];

export const AUTH_STRENGTHS = ["NONE", "SINGLE_FACTOR", "MULTI_FACTOR", "STEP_UP"] as const;
export type AuthStrength = (typeof AUTH_STRENGTHS)[number];

const SESSION_FIELDS = new Set<string>([
  "session_id",
  "principal_id",
  "tenant_id",
  "device_id",
  "grant_flow",
  "strength",
  "created_at_unix_s",
  "expires_at_unix_s",
  "revoked",
  "correlation",
]);

export interface AuthSessionShape {
  session_id: string;
  principal_id: string;
  tenant_id: string;
  device_id: string;
  grant_flow: GrantFlow;
  strength: AuthStrength;
  created_at_unix_s: number;
  expires_at_unix_s: number;
  revoked: boolean;
  correlation: string;
}

export enum SessionStatus {
  ACTIVE = "ACTIVE",
  EXPIRED = "EXPIRED",
  REVOKED = "REVOKED",
}

export class AuthenticatedSession {
  readonly session_id: string;
  readonly principal_id: string;
  readonly tenant_id: string;
  readonly device_id: string;
  readonly grant_flow: GrantFlow;
  readonly strength: AuthStrength;
  readonly created_at_unix_s: number;
  readonly expires_at_unix_s: number;
  readonly revoked: boolean;
  readonly correlation: string;

  private constructor(shape: AuthSessionShape) {
    this.session_id = shape.session_id;
    this.principal_id = shape.principal_id;
    this.tenant_id = shape.tenant_id;
    this.device_id = shape.device_id;
    this.grant_flow = shape.grant_flow;
    this.strength = shape.strength;
    this.created_at_unix_s = shape.created_at_unix_s;
    this.expires_at_unix_s = shape.expires_at_unix_s;
    this.revoked = shape.revoked;
    this.correlation = shape.correlation;
  }

  /** Validate wire-shaped input with deny-unknown semantics. */
  static fromWire(value: unknown): AuthenticatedSession {
    const obj = assertObject(value, "AuthSession");
    rejectUnknownFields(obj, SESSION_FIELDS, "AuthSession");
    return new AuthenticatedSession({
      session_id: assertUuid(obj.session_id, "session_id"),
      principal_id: assertUuid(obj.principal_id, "principal_id"),
      tenant_id: assertUuid(obj.tenant_id, "tenant_id"),
      device_id: assertUuid(obj.device_id, "device_id"),
      grant_flow: assertEnum(obj.grant_flow, new Set<GrantFlow>(GRANT_FLOWS), "grant_flow"),
      strength: assertEnum(obj.strength, new Set<AuthStrength>(AUTH_STRENGTHS), "strength"),
      created_at_unix_s: assertInt(obj.created_at_unix_s, "created_at_unix_s"),
      expires_at_unix_s: assertInt(obj.expires_at_unix_s, "expires_at_unix_s"),
      revoked: assertBool(obj.revoked, "revoked"),
      correlation: assertUuid(obj.correlation, "correlation"),
    });
  }

  /** Status is computed from authoritative fields, never cached UI state. */
  statusAt(nowUnixS: number): SessionStatus {
    if (this.revoked) {
      return SessionStatus.REVOKED;
    }
    if (nowUnixS >= this.expires_at_unix_s) {
      return SessionStatus.EXPIRED;
    }
    return SessionStatus.ACTIVE;
  }

  /**
   * Fail-closed gate for consequential mutation (directive N).
   * An expired/revoked session refuses the mutation and NEVER queues
   * it for blind replay: the caller must re-authenticate and re-issue.
   */
  requireActive(nowUnixS: number, correlationId: string): void {
    const status = this.statusAt(nowUnixS);
    if (status === SessionStatus.ACTIVE) {
      return;
    }
    throw new Spec006Error(
      status === SessionStatus.EXPIRED
        ? ErrorCode.Authentication
        : ErrorCode.Authorization,
      status === SessionStatus.EXPIRED
        ? "Session expired; re-authenticate before issuing consequential actions"
        : "Session revoked; re-authenticate before issuing consequential actions",
      correlationId,
    );
  }
}

/**
 * Business/workspace context binding (directive F). Mirrors the
 * canonical Hydra business-context schema: tenant_id, principal_id,
 * scope, business_id, correlation. The business id is optional on the
 * wire (a principal may have no active business), but when present it
 * is a UUID and is part of the authority binding.
 */
export const BUSINESS_SCOPES = ["GLOBAL", "BUSINESS", "PERSONAL"] as const;
export type BusinessScope = (typeof BUSINESS_SCOPES)[number];

const BUSINESS_CONTEXT_FIELDS = new Set<string>([
  "tenant_id",
  "principal_id",
  "scope",
  "business_id",
  "correlation",
]);

export interface BusinessContextShape {
  tenant_id: string;
  principal_id: string;
  scope: BusinessScope;
  business_id: string | undefined;
  correlation: string;
}

export class BusinessContext {
  readonly tenant_id: string;
  readonly principal_id: string;
  readonly scope: BusinessScope;
  readonly business_id: string | undefined;
  readonly correlation: string;

  private constructor(shape: BusinessContextShape) {
    this.tenant_id = shape.tenant_id;
    this.principal_id = shape.principal_id;
    this.scope = shape.scope;
    this.business_id = shape.business_id;
    this.correlation = shape.correlation;
  }

  static fromWire(value: unknown): BusinessContext {
    const obj = assertObject(value, "BusinessContext");
    rejectUnknownFields(obj, BUSINESS_CONTEXT_FIELDS, "BusinessContext");
    const businessId = obj.business_id === undefined ? undefined : assertUuid(obj.business_id, "business_id");
    return new BusinessContext({
      tenant_id: assertUuid(obj.tenant_id, "tenant_id"),
      principal_id: assertUuid(obj.principal_id, "principal_id"),
      scope: assertEnum(obj.scope, new Set<BusinessScope>(BUSINESS_SCOPES), "scope"),
      business_id: businessId as string | undefined,
      correlation: assertUuid(obj.correlation, "correlation"),
    });
  }

  /** A projection is valid only when bound to this exact context. */
  binds(tenantId: string, principalId: string, businessId: string | undefined): boolean {
    if (this.tenant_id !== tenantId || this.principal_id !== principalId) {
      return false;
    }
    if (this.business_id === undefined || businessId === undefined) {
      return this.business_id === businessId;
    }
    return this.business_id === businessId;
  }
}

/**
 * Bound dashboard context: session + business workspace.
 * Context switching constructs a NEW bound context; projections of the
 * previous context are invalidated (directive G). A stale tab that
 * still holds the old context cannot act on the new one.
 */
export class BoundContext {
  readonly session: AuthenticatedSession;
  readonly business: BusinessContext;

  private constructor(session: AuthenticatedSession, business: BusinessContext) {
    this.session = session;
    this.business = business;
  }

  static bind(session: AuthenticatedSession, business: BusinessContext): BoundContext {
    if (session.principal_id !== business.principal_id) {
      throw new Spec006Error(
        ErrorCode.Authorization,
        "Business context principal does not match session principal",
      );
    }
    if (session.tenant_id !== business.tenant_id) {
      throw new Spec006Error(
        ErrorCode.Authorization,
        "Business context tenant does not match session tenant",
      );
    }
    return new BoundContext(session, business);
  }

  /** Returns a new bound context; the receiver is NOT mutated. */
  switchBusiness(next: BusinessContext): BoundContext {
    return BoundContext.bind(this.session, next);
  }
}

/**
 * A context-bound projection: any dashboard data rendered for a
 * specific business context. After a context switch the old projection
 * is invalid; consequential actions on an invalid projection are
 * refused (directive G/J).
 */
export class ContextProjection<T> {
  readonly context: BoundContext;
  readonly data: T;
  private invalidated = false;

  constructor(context: BoundContext, data: T) {
    this.context = context;
    this.data = data;
  }

  invalidate(): void {
    this.invalidated = true;
  }

  /**
   * Guard for using this projection. After the owning context was
   * switched away, any use fails closed.
   */
  requireCurrent(active: BoundContext): T {
    if (this.invalidated) {
      throw new Spec006Error(
        ErrorCode.Conflict,
        "Projection was invalidated by a context switch",
        active.session.correlation,
      );
    }
    if (
      !active.business.binds(
        this.context.business.tenant_id,
        this.context.business.principal_id,
        this.context.business.business_id,
      )
    ) {
      throw new Spec006Error(
        ErrorCode.Conflict,
        "Projection belongs to a different business context",
        active.session.correlation,
      );
    }
    return this.data;
  }
}
