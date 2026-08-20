/**
 * EP-033 M3 integration: StatusBadge and DashboardShellView through
 * REAL React rendering. Connectivity/freshness and context binding
 * must survive to rendered output; stale is never presented as live
 * (directive F/I/J).
 */

import { describe, expect, it } from "vitest";
import { renderToString } from "react-dom/server";
import {
  AuthenticatedSession,
  BoundContext,
  BusinessContext,
  DashboardShell,
} from "@nexus/web";
import { StatusBadge } from "../components/status-badge";
import { DashboardShellView } from "../components/dashboard-shell-view";

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

describe("ep033_integration_status", () => {
  it("renders connectivity verbatim with a status role", () => {
    const html = renderToString(
      <StatusBadge connectivity="OFFLINE" freshness="STALE" />,
    );
    expect(html).toContain('role="status"');
    expect(html).toContain('data-connectivity="OFFLINE"');
    expect(html).toContain('data-freshness="STALE"');
    expect(html).toContain("OFFLINE");
    expect(html).toContain("(stale)");
  });

  it("labels stale data explicitly, never as live", () => {
    const html = renderToString(
      <StatusBadge connectivity="CONNECTED" freshness="STALE" />,
    );
    expect(html).toContain("(stale)");
    expect(html).not.toContain("CONNECTED</span>");
  });

  it("renders fresh connected state without a stale label", () => {
    const html = renderToString(
      <StatusBadge connectivity="CONNECTED" freshness="FRESH" />,
    );
    expect(html).toContain("CONNECTED");
    expect(html).not.toContain("stale");
  });

  it("conveys state by text, not color alone (non-color status)", () => {
    const html = renderToString(
      <StatusBadge connectivity="AUTH_EXPIRED" freshness="STALE" />,
    );
    expect(html).toContain("AUTH_EXPIRED");
    // No color-only encoding exists in the rendered output.
    expect(html).not.toContain("style=");
  });
});

describe("ep033_integration_shell", () => {
  it("binds tenant/principal/business from the session context, not a label", () => {
    const context = bound();
    const shell = DashboardShell.create(
      "approvals",
      "approvals",
      "CONNECTED",
      context,
    );
    const html = renderToString(
      <DashboardShellView
        shell={shell}
        context={context}
        connectivity="CONNECTED"
      />,
    );
    expect(html).toContain(`data-tenant-id="${uuid(3)}"`);
    expect(html).toContain(`data-principal-id="${uuid(2)}"`);
    expect(html).toContain(`data-business-id="${uuid(6)}"`);
  });

  it("renders the current route and surface", () => {
    const context = bound();
    const shell = DashboardShell.create(
      "security",
      "security",
      "CONNECTED",
      context,
    );
    const html = renderToString(
      <DashboardShellView
        shell={shell}
        context={context}
        connectivity="DEGRADED"
      />,
    );
    expect(html).toContain('data-route="security"');
    expect(html).toContain('data-surface="security"');
    expect(html).toContain('data-connectivity="DEGRADED"');
  });

  it("renders primary navigation landmarks", () => {
    const context = bound();
    const shell = DashboardShell.create(
      "home",
      "dashboard",
      "CONNECTED",
      context,
    );
    const html = renderToString(
      <DashboardShellView
        shell={shell}
        context={context}
        connectivity="CONNECTED"
      />,
    );
    expect(html).toContain('role="navigation"');
    expect(html).toContain("Approvals");
    expect(html).toContain("Security");
  });
});
