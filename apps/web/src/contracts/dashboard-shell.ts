/**
 * EP-033 M1 DashboardShell contract (SPEC-004 behavior 5).
 *
 * The shell binds a bound session/context, a connectivity state, and a
 * navigation model over the canonical dashboard vocabulary (chat,
 * voice status, objectives, agents, approvals, home, security,
 * businesses, social, communications, fleet, costs, incidents, memory,
 * skills, integrations). Unknown routes fail closed: an unsupported
 * route is an explicit UNSUPPORTED state, never a best-effort
 * fabricated panel (directive E).
 */

import {
  assertEnum,
  assertObject,
  assertString,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";
import type { BoundContext } from "./session";
import type { ConnectivityState } from "./state";
import { CONNECTIVITY_STATES } from "./state";

/** Canonical dashboard navigation vocabulary (SPEC-004 behavior 5). */
export const DASHBOARD_ROUTES = [
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
] as const;
export type DashboardRoute = (typeof DASHBOARD_ROUTES)[number];

export const DASHBOARD_SURFACES = [
  "dashboard",
  "chat",
  "objectives",
  "approvals",
  "fleet",
  "security",
  "provider_settings",
  "audit",
] as const;
export type DashboardSurface = (typeof DASHBOARD_SURFACES)[number];

const SHELL_STATE_FIELDS = new Set<string>([
  "route",
  "surface",
  "connectivity",
  "correlation",
]);

export interface DashboardShellShape {
  route: DashboardRoute;
  surface: DashboardSurface;
  connectivity: ConnectivityState;
  correlation: string;
}

/**
 * The dashboard shell's own state. It carries the current navigation
 * route and surface, but NEVER carries tenant/principal/business
 * context: authority comes from the bound session context, not from
 * which screen is selected (directive F).
 */
export class DashboardShell {
  readonly route: DashboardRoute;
  readonly surface: DashboardSurface;
  readonly connectivity: ConnectivityState;
  readonly correlation: string;
  readonly context: BoundContext;

  private constructor(shape: DashboardShellShape, context: BoundContext) {
    this.route = shape.route;
    this.surface = shape.surface;
    this.connectivity = shape.connectivity;
    this.correlation = shape.correlation;
    this.context = context;
  }

  static create(
    route: DashboardRoute,
    surface: DashboardSurface,
    connectivity: ConnectivityState,
    context: BoundContext,
    correlation?: string,
  ): DashboardShell {
    return new DashboardShell(
      {
        route,
        surface,
        connectivity,
        correlation: correlation ?? context.session.correlation,
      },
      context,
    );
  }

  static fromWire(value: unknown, context: BoundContext): DashboardShell {
    const obj = assertObject(value, "DashboardShell");
    rejectUnknownFields(obj, SHELL_STATE_FIELDS, "DashboardShell");
    return new DashboardShell(
      {
        route: assertEnum(
          obj.route,
          new Set<DashboardRoute>(DASHBOARD_ROUTES),
          "route",
        ),
        surface: assertEnum(
          obj.surface,
          new Set<DashboardSurface>(DASHBOARD_SURFACES),
          "surface",
        ),
        connectivity: assertEnum(
          obj.connectivity,
          new Set<ConnectivityState>(CONNECTIVITY_STATES),
          "connectivity",
        ),
        correlation: assertString(obj.correlation, "correlation"),
      },
      context,
    );
  }

  /**
   * Navigation with unknown-route fail-closed (directive E): the shell
   * never fabricates a panel for a route outside the canonical
   * vocabulary.
   */
  static navigate(
    requested: string,
    context: BoundContext,
  ): { shell: DashboardShell; unsupported: boolean } {
    if ((DASHBOARD_ROUTES as readonly string[]).includes(requested)) {
      const route = requested as DashboardRoute;
      const surface: DashboardSurface =
        route === "chat"
          ? "chat"
          : route === "objectives"
            ? "objectives"
            : route === "approvals"
              ? "approvals"
              : route === "fleet"
                ? "fleet"
                : route === "security"
                  ? "security"
                  : route === "integrations"
                    ? "provider_settings"
                    : route === "incidents"
                      ? "audit"
                      : "dashboard";
      return {
        shell: DashboardShell.create(route, surface, "CONNECTED", context),
        unsupported: false,
      };
    }
    return {
      shell: DashboardShell.create("home", "dashboard", "CONNECTED", context),
      unsupported: true,
    };
  }
}
