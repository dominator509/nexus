/**
 * EP-033 PWA entry (AUD-038).
 *
 * This is the ACTUAL React PWA entry point: a real index.html + Vite
 * production build mounting the REAL @nexus/ui React components over
 * REAL @nexus/web contract state through ReactDOM. Previously the web
 * package was only a framework-neutral contract layer and the
 * accessibility proof scanned a server-rendered fixture document -
 * no owned browser entry existed. This entry is that missing surface.
 *
 * The PWA renders contract state; it NEVER mints authority. All
 * consequential actions flow through the typed command/approval
 * contracts (@nexus/web), and the components fail closed on unknown
 * or unauthorized capabilities (directive D/E).
 */
import { createRoot } from "react-dom/client";
import {
  ApprovalCard,
  AuthenticatedSession,
  BoundContext,
  BusinessContext,
  DashboardShell,
  PresentedCapability,
} from "@nexus/web";
import {
  ApprovalCardView,
  CapabilityButton,
  DashboardShellView,
  StatusBadge,
} from "@nexus/ui";

function uuid(n: number): string {
  return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
}

/** Contract fixtures bound to a real session (display-only state). */
function shellState(): {
  shell: DashboardShell;
  context: BoundContext;
} {
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
  const context = BoundContext.bind(
    session,
    BusinessContext.fromWire({
      tenant_id: uuid(3),
      principal_id: uuid(2),
      scope: "BUSINESS",
      business_id: uuid(10),
      correlation: uuid(5),
    }),
  );
  return {
    shell: DashboardShell.create("approvals", "approvals", "CONNECTED", context),
    context,
  };
}

function approvalCard(): ApprovalCard {
  return ApprovalCard.fromWire({
    approval_id: uuid(30),
    action_id: uuid(31),
    capability_id: "sentinel.contain.quarantine",
    approval_class: "FOUR_EYES",
    risk: "R3",
    action_label: "Quarantine host",
    target: "host:edge-01",
    external_effects: "Blocks network egress",
    cost: "0",
    reversibility: "REVERSIBLE",
    requester_id: uuid(32),
    expires_at_unix_s: 1_800_000_000,
    correlation: uuid(33),
  });
}

function capabilities(): Array<PresentedCapability> {
  return [
    PresentedCapability.fromWire({
      capability_id: "home.lights.query",
      class: "QUERY",
      availability: "AVAILABLE",
      visible: true,
      authorized: true,
      required_approval: "NONE",
    }),
    PresentedCapability.fromWire({
      capability_id: "home.lights.set",
      class: "COMMAND",
      availability: "AVAILABLE",
      visible: true,
      authorized: false,
      required_approval: "HUMAN",
    }),
    PresentedCapability.fromWire({
      capability_id: "sentinel.contain.quarantine",
      class: "COMMAND",
      availability: "AVAILABLE",
      visible: true,
      authorized: true,
      required_approval: "FOUR_EYES",
    }),
  ];
}

const { shell, context } = shellState();
const root = document.getElementById("root");
if (root === null) {
  throw new Error("PWA root element missing");
}

createRoot(root).render(
  <main id="main">
    <DashboardShellView shell={shell} context={context} connectivity="CONNECTED" />
    <section aria-label="Capabilities">
      {capabilities().map((capability) => (
        <CapabilityButton
          key={capability.capability_id}
          capability={capability}
          label={capability.capability_id}
        />
      ))}
    </section>
    <ApprovalCardView
      card={approvalCard()}
      state="PENDING"
      distinctApprovers={[uuid(40)]}
    />
    <StatusBadge connectivity="CONNECTED" freshness="FRESH" />
    <StatusBadge connectivity="OFFLINE" freshness="STALE" />
  </main>,
);
