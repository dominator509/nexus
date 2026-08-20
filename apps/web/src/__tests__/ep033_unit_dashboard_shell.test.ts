import { describe, expect, it } from "vitest";
import {
  DashboardShell,
  DASHBOARD_ROUTES,
  DASHBOARD_SURFACES,
} from "../contracts/dashboard-shell";
import {
  AuthenticatedSession,
  BusinessContext,
  BoundContext,
} from "../contracts/session";

function uuid(n: number): string {
  return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
}

function bound(): BoundContext {
  const session = AuthenticatedSession.fromWire({
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
  });
  const business = BusinessContext.fromWire({
    tenant_id: uuid(3),
    principal_id: uuid(2),
    scope: "BUSINESS",
    business_id: uuid(6),
    correlation: uuid(7),
  });
  return BoundContext.bind(session, business);
}

describe("ep033_unit_dashboard_shell", () => {
  it("covers the canonical SPEC-004 dashboard vocabulary", () => {
    expect([...DASHBOARD_ROUTES]).toEqual([
      "chat",
      "voice_status",
      "objectives",
      "agents",
      "approvals",
      "home",
      "security",
      "businesses",
      "social",
      "communications",
      "fleet",
      "costs",
      "incidents",
      "memory",
      "skills",
      "integrations",
    ]);
    expect([...DASHBOARD_SURFACES]).toEqual([
      "dashboard",
      "chat",
      "objectives",
      "approvals",
      "fleet",
      "security",
      "provider_settings",
      "audit",
    ]);
  });

  it("binds shell state to the session context, not to a screen label", () => {
    const context = bound();
    const shell = DashboardShell.create("chat", "chat", "CONNECTED", context);
    expect(shell.context.session.principal_id).toBe(uuid(2));
    expect(shell.context.business.tenant_id).toBe(uuid(3));
  });

  it("navigates canonical routes to their surfaces", () => {
    const context = bound();
    const result = DashboardShell.navigate("approvals", context);
    expect(result.unsupported).toBe(false);
    expect(result.shell.surface).toBe("approvals");
    expect(result.shell.route).toBe("approvals");
  });

  it("fails closed on unknown routes (never fabricated panel)", () => {
    const context = bound();
    const result = DashboardShell.navigate("magic-panel", context);
    expect(result.unsupported).toBe(true);
  });

  it("maps incident and integration routes to audit and settings surfaces", () => {
    const context = bound();
    expect(DashboardShell.navigate("incidents", context).shell.surface).toBe(
      "audit",
    );
    expect(DashboardShell.navigate("integrations", context).shell.surface).toBe(
      "provider_settings",
    );
  });

  it("rejects unknown fields in shell wire input", () => {
    const context = bound();
    expect(() =>
      DashboardShell.fromWire(
        {
          route: "chat",
          surface: "chat",
          connectivity: "CONNECTED",
          correlation: uuid(5),
          selected_business_label: "Acme",
        },
        context,
      ),
    ).toThrowError();
  });

  it("rejects unknown routes in shell wire input", () => {
    const context = bound();
    expect(() =>
      DashboardShell.fromWire(
        {
          route: "magic-panel",
          surface: "chat",
          connectivity: "CONNECTED",
          correlation: uuid(5),
        },
        context,
      ),
    ).toThrowError();
  });
});
