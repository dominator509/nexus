/**
 * EP-033 M5 accessibility harness: renders the REAL @nexus/ui production
 * components (DashboardShellView, ApprovalCardView, StatusBadge,
 * CapabilityButton, ChatComposer) over REAL @nexus/web contract state
 * through react-dom/server into a complete HTML document.
 *
 * This is the actual rendered UI for the owned surfaces - the same
 * server-render path the M3 integration suite proves. The accessibility
 * scan loads THIS document in real headless Chrome and runs the real
 * axe-core WCAG 2.2 AA rule set against the real DOM. No emulation, no
 * jsdom, no component doubles.
 */

import { renderToString } from "react-dom/server";
import {
  ApprovalCard,
  AuthenticatedSession,
  BoundContext,
  BusinessContext,
  DashboardShell,
  KnownCapabilityVocabulary,
  PresentedCapability,
} from "@nexus/web";
import {
  ApprovalCardView,
  CapabilityButton,
  ChatComposer,
  DashboardShellView,
  StatusBadge,
} from "@nexus/ui";

function uuid(n: number): string {
  return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
}

export function sessionFixture(): AuthenticatedSession {
  return AuthenticatedSession.fromWire({
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
}

export function contextFixture(): BoundContext {
  return BoundContext.bind(
    sessionFixture(),
    BusinessContext.fromWire({
      tenant_id: uuid(3),
      principal_id: uuid(2),
      scope: "BUSINESS",
      business_id: uuid(10),
      correlation: uuid(5),
    }),
  );
}

export function shellFixture(): DashboardShell {
  return DashboardShell.create(
    "approvals",
    "approvals",
    "CONNECTED",
    contextFixture(),
  );
}

export function approvalCardFixture(): ApprovalCard {
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

export function capabilitiesFixture(): Array<PresentedCapability> {
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

/**
 * Render the owned surfaces into a complete, standards-shaped HTML
 * document. The harness adds only document scaffolding (html/head/
 * main/skip link); every interactive and informational element comes
 * from the real production components.
 */
export function renderOwnedSurfacesHtml(): string {
  const context = contextFixture();
  const shell = shellFixture();
  const card = approvalCardFixture();
  const capabilities = capabilitiesFixture();
  const body = renderToString(
    <main id="main">
      <DashboardShellView shell={shell} context={context} connectivity="CONNECTED" />
      <section aria-label="Capabilities">
        {capabilities.map((capability) => (
          <CapabilityButton
            key={capability.capability_id}
            capability={capability}
            label={capability.capability_id}
          />
        ))}
      </section>
      <ApprovalCardView card={card} state="PENDING" distinctApprovers={[uuid(40)]} />
      <StatusBadge connectivity="CONNECTED" freshness="FRESH" />
      <StatusBadge connectivity="OFFLINE" freshness="STALE" />
      <ChatComposer
        conversationId="conv-lf005"
        correlationId={uuid(5)}
        onSend={() => {}}
      />
    </main>,
  );
  return [
    "<!doctype html>",
    '<html lang="en">',
    "<head>",
    '<meta charset="utf-8" />',
    '<meta name="viewport" content="width=device-width, initial-scale=1" />',
    "<title>Nexus owned surfaces (EP-033 M5 a11y scan)</title>",
    "</head>",
    "<body>",
    '<a href="#main" class="skip-link">Skip to main content</a>',
    body,
    "</body>",
    "</html>",
  ].join("\n");
}
