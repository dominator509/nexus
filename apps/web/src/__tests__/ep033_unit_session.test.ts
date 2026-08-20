import { describe, expect, it } from "vitest";
import {
  AuthenticatedSession,
  BusinessContext,
  BoundContext,
  ContextProjection,
  SessionStatus,
} from "../contracts/session";
import { ErrorCode, Spec006Error } from "../contracts/errors";

function uuid(n: number): string {
  return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
}

function sessionWire(
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  return {
    session_id: uuid(1),
    principal_id: uuid(2),
    tenant_id: uuid(3),
    device_id: uuid(4),
    grant_flow: "AUTHORIZATION_CODE",
    strength: "MULTI_FACTOR",
    created_at_unix_s: 1_700_000_000,
    expires_at_unix_s: 1_800_000_000,
    revoked: false,
    correlation: uuid(5),
    ...overrides,
  };
}

function businessWire(
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  return {
    tenant_id: uuid(3),
    principal_id: uuid(2),
    scope: "BUSINESS",
    business_id: uuid(6),
    correlation: uuid(7),
    ...overrides,
  };
}

describe("ep033_unit_session", () => {
  it("constructs a valid authenticated session from canonical wire shape", () => {
    const session = AuthenticatedSession.fromWire(sessionWire());
    expect(session.principal_id).toBe(uuid(2));
    expect(session.tenant_id).toBe(uuid(3));
    expect(session.grant_flow).toBe("AUTHORIZATION_CODE");
    expect(session.strength).toBe("MULTI_FACTOR");
    expect(session.statusAt(1_700_000_001)).toBe(SessionStatus.ACTIVE);
  });

  it("rejects unknown fields (deny-unknown, canonical schema semantics)", () => {
    const wire = sessionWire({ injected: "x" });
    expect(() => AuthenticatedSession.fromWire(wire)).toThrowError(
      Spec006Error,
    );
    try {
      AuthenticatedSession.fromWire(wire);
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Validation);
    }
  });

  it("rejects invalid enum values and malformed uuids", () => {
    expect(() =>
      AuthenticatedSession.fromWire(sessionWire({ grant_flow: "MAGIC" })),
    ).toThrowError(Spec006Error);
    expect(() =>
      AuthenticatedSession.fromWire(sessionWire({ session_id: "not-a-uuid" })),
    ).toThrowError(Spec006Error);
  });

  it("computes EXPIRED from authoritative expiry, not cached state", () => {
    const session = AuthenticatedSession.fromWire(sessionWire());
    expect(session.statusAt(1_900_000_000)).toBe(SessionStatus.EXPIRED);
  });

  it("computes REVOKED before expiry", () => {
    const session = AuthenticatedSession.fromWire(
      sessionWire({ revoked: true }),
    );
    expect(session.statusAt(1_700_000_001)).toBe(SessionStatus.REVOKED);
  });

  it("fails closed on expired session for consequential mutation", () => {
    const session = AuthenticatedSession.fromWire(sessionWire());
    try {
      session.requireActive(1_900_000_000, uuid(8));
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Authentication);
    }
  });

  it("binds business context to session principal and tenant", () => {
    const session = AuthenticatedSession.fromWire(sessionWire());
    const business = BusinessContext.fromWire(businessWire());
    const bound = BoundContext.bind(session, business);
    expect(bound.business.business_id).toBe(uuid(6));
  });

  it("refuses binding a business context from another principal", () => {
    const session = AuthenticatedSession.fromWire(sessionWire());
    const other = BusinessContext.fromWire(
      businessWire({ principal_id: uuid(9) }),
    );
    expect(() => BoundContext.bind(session, other)).toThrowError(Spec006Error);
  });

  it("invalidates prior-context projections on business switch", () => {
    const session = AuthenticatedSession.fromWire(sessionWire());
    const businessA = BusinessContext.fromWire(
      businessWire({ business_id: uuid(6) }),
    );
    const boundA = BoundContext.bind(session, businessA);
    const projectionA = new ContextProjection<string>(boundA, "A-only data");

    const businessB = BusinessContext.fromWire(
      businessWire({ business_id: uuid(10) }),
    );
    const boundB = boundA.switchBusiness(businessB);
    projectionA.invalidate();

    // A-only data must no longer be actionable after the switch.
    expect(() => projectionA.requireCurrent(boundB)).toThrowError(Spec006Error);
    try {
      projectionA.requireCurrent(boundB);
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Conflict);
    }
  });

  it("refuses using a projection bound to a different business context", () => {
    const session = AuthenticatedSession.fromWire(sessionWire());
    const businessA = BusinessContext.fromWire(
      businessWire({ business_id: uuid(6) }),
    );
    const boundA = BoundContext.bind(session, businessA);
    const projectionA = new ContextProjection<string>(boundA, "A-only data");

    const businessB = BusinessContext.fromWire(
      businessWire({ business_id: uuid(10) }),
    );
    const boundB = BoundContext.bind(session, businessB);
    // Without explicit invalidation, context mismatch alone refuses use.
    expect(() => projectionA.requireCurrent(boundB)).toThrowError(Spec006Error);
  });

  it("allows use of a projection while its context is current", () => {
    const session = AuthenticatedSession.fromWire(sessionWire());
    const businessA = BusinessContext.fromWire(
      businessWire({ business_id: uuid(6) }),
    );
    const boundA = BoundContext.bind(session, businessA);
    const projectionA = new ContextProjection<string>(boundA, "A-only data");
    expect(projectionA.requireCurrent(boundA)).toBe("A-only data");
  });
});
