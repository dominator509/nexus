/**
 * DashboardShellView - EP-033 M3 shared UI component.
 *
 * Binds the dashboard shell contract to a bound session context: the
 * shell renders the current route/surface and connectivity, and it
 * NEVER derives tenant/principal/business authority from a screen
 * label (directive F). Consequential actions always flow through the
 * typed command/approval contracts.
 */

import type { BoundContext, DashboardShell, ConnectivityState } from "@nexus/web";

export interface DashboardShellViewProps {
  shell: DashboardShell;
  context: BoundContext;
  connectivity: ConnectivityState;
}

export function DashboardShellView(props: DashboardShellViewProps): React.ReactElement {
  const { shell, context, connectivity } = props;
  return (
    <div
      role="navigation"
      aria-label="Dashboard shell"
      data-route={shell.route}
      data-surface={shell.surface}
      data-connectivity={connectivity}
    >
      <span data-tenant-id={context.business.tenant_id} />
      <span data-principal-id={context.session.principal_id} />
      <span data-business-id={context.business.business_id ?? "none"} />
      <nav aria-label="Primary">
        <ul>
          <li>Chat</li>
          <li>Objectives</li>
          <li>Approvals</li>
          <li>Fleet</li>
          <li>Security</li>
        </ul>
      </nav>
    </div>
  );
}
