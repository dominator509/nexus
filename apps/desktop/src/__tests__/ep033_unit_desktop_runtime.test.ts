import { describe, expect, it } from "vitest";
import {
  AuthenticatedSession,
  BoundContext,
  BusinessContext,
  ErrorCode,
  SessionStatus,
  Spec006Error,
} from "@nexus/web";
import { DesktopShellRuntime } from "../runtime";

function uuid(n: number): string {
  return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
}

function session(
  expiresAt = 1_800_000_000,
  revoked = false,
): AuthenticatedSession {
  return AuthenticatedSession.fromWire({
    session_id: uuid(1),
    principal_id: uuid(2),
    tenant_id: uuid(3),
    device_id: uuid(4),
    grant_flow: "AUTHORIZATION_CODE",
    strength: "MULTI_FACTOR",
    created_at_unix_s: 1_700_000_000,
    expires_at_unix_s: expiresAt,
    revoked,
    correlation: uuid(5),
  });
}

function business(businessId: string): BusinessContext {
  return BusinessContext.fromWire({
    tenant_id: uuid(3),
    principal_id: uuid(2),
    scope: "BUSINESS",
    business_id: businessId,
    correlation: uuid(7),
  });
}

function bound(expiresAt = 1_800_000_000): BoundContext {
  return BoundContext.bind(session(expiresAt), business(uuid(6)));
}

describe("ep033_unit_desktop_runtime", () => {
  it("binds the runtime to session and business context", () => {
    const runtime = new DesktopShellRuntime(bound());
    expect(runtime.context.session.principal_id).toBe(uuid(2));
    expect(runtime.context.business.business_id).toBe(uuid(6));
    expect(runtime.snapshot(1_700_000_001).status).toBe("ACTIVE");
  });

  it("reports AUTH_EXPIRED when the session expired", () => {
    const runtime = new DesktopShellRuntime(bound(1_000_000_000));
    const snapshot = runtime.snapshot(1_700_000_001);
    expect(snapshot.status).toBe("AUTH_EXPIRED");
    expect(snapshot.correlation).toBe(uuid(5));
  });

  it("reflects connectivity transitions in the snapshot", () => {
    const runtime = new DesktopShellRuntime(bound());
    runtime.setConnectivity("OFFLINE");
    expect(runtime.snapshot(1_700_000_001).status).toBe("OFFLINE");
    runtime.setConnectivity("CONNECTED");
    expect(runtime.snapshot(1_700_000_001).status).toBe("ACTIVE");
  });

  it("invalidates prior-context projections on business switch", () => {
    const runtime = new DesktopShellRuntime(bound());
    const projection = runtime.project("A-only data");
    const nextBusiness = business(uuid(10));
    const next = runtime.switchBusiness(nextBusiness);
    expect(next.business.business_id).toBe(uuid(10));
    expect(() => projection.requireCurrent(runtime.context)).toThrowError(
      Spec006Error,
    );
  });

  it("refuses consequential actions under an expired session", () => {
    const runtime = new DesktopShellRuntime(bound(1_000_000_000));
    try {
      runtime.requireConsequential(1_700_000_001, "delete-host");
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Authentication);
    }
  });

  it("refuses consequential actions while the backend is unavailable", () => {
    const runtime = new DesktopShellRuntime(bound());
    runtime.setConnectivity("BACKEND_UNAVAILABLE");
    try {
      runtime.requireConsequential(1_700_000_001, "send-command");
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Unavailable);
    }
  });

  it("permits consequential actions with active session and connectivity", () => {
    const runtime = new DesktopShellRuntime(bound());
    expect(() =>
      runtime.requireConsequential(1_700_000_001, "send-command"),
    ).not.toThrow();
  });

  it("labels offline-fetched payloads as stale, never actionable", () => {
    const runtime = new DesktopShellRuntime(bound());
    runtime.setConnectivity("OFFLINE");
    const view = runtime.labelPayload({ value: 1 }, "corr-1");
    expect(view.freshness).toBe("STALE");
    expect(() => view.requireFresh("mutate")).toThrowError(Spec006Error);
  });

  it("labels connected payloads as fresh and actionable", () => {
    const runtime = new DesktopShellRuntime(bound());
    const view = runtime.labelPayload({ value: 1 }, "corr-1");
    expect(view.freshness).toBe("FRESH");
    expect(view.requireFresh("mutate")).toEqual({ value: 1 });
  });

  it("rejects non-monotonic revalidation", () => {
    const runtime = new DesktopShellRuntime(bound());
    const stale = runtime.labelPayload({ value: 1 }, "corr-1");
    const older = runtime.labelPayload({ value: 0 }, "corr-1", { revision: 0 });
    expect(() => runtime.revalidate(stale, older)).toThrowError(Spec006Error);
  });

  it("theme application never mutates runtime authority state", () => {
    const runtime = new DesktopShellRuntime(bound());
    const authority = { tenant: uuid(3), capabilities: ["home.lights.query"] };
    expect(runtime.applyAppearance(authority)).toBe(authority);
  });
});
