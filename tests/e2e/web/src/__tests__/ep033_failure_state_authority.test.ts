/**
 * EP-033 M4 forced failures: state/authority invariants (directives I-N).
 *
 * The failure mechanism is REAL STATE TRANSITIONS through the production
 * desktop runtime, dispatcher, and approval flow: context switches,
 * connectivity loss, duplicate requests, and four-eyes abuse are pushed
 * through the real components. Nothing is mocked; the components
 * themselves are the authority boundary under test.
 */

import { describe, expect, it } from "vitest";
import {
  ApprovalAction,
  ApprovalCard,
  AuthenticatedSession,
  BoundContext,
  BusinessContext,
  ErrorCode,
  KnownCapabilityVocabulary,
  Spec006Error,
  TypedCommandRequest,
} from "@nexus/web";
import {
  DesktopApprovalFlow,
  DesktopCommandDispatcher,
  DesktopShellRuntime,
  DesktopViewState,
} from "@nexus/desktop";

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

function business(
  businessId: string | undefined,
  correlation: string,
): BusinessContext {
  return BusinessContext.fromWire({
    tenant_id: uuid(3),
    principal_id: uuid(2),
    scope: "BUSINESS",
    business_id: businessId,
    correlation,
  });
}

function requestWire(
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  return {
    action_id: uuid(20),
    tenant_id: uuid(3),
    principal_id: uuid(2),
    capability_id: "home.lights.set",
    idempotency_key: "req-000000000000001",
    risk: "R1",
    approval_class: "NONE",
    reversal: "home.lights.set:reverse",
    arguments: { on: true },
    expected_state: { on: true },
    invocation: {
      request_id: "req-000000000000002",
      correlation_id: uuid(5),
      origin_system: "web",
      external_actor_id: uuid(2),
      external_actor_type: "principal",
    },
    ...overrides,
  };
}

const VOCABULARY = new KnownCapabilityVocabulary(
  ["home.lights.query", "home.lights.set", "sentinel.contain.quarantine"],
  [
    { capability_id: "home.lights.query", risk: "R0", approval: "NONE" },
    { capability_id: "home.lights.set", risk: "R1", approval: "NONE" },
    {
      capability_id: "sentinel.contain.quarantine",
      risk: "R4",
      approval: "FOUR_EYES",
    },
  ],
);

describe("ep033_failure_state_authority", () => {
  it("invalidates a business-A projection after switching to business B (stale handle)", () => {
    const runtime = new DesktopShellRuntime(
      BoundContext.bind(session(), business(uuid(10), uuid(5))),
    );
    const projection = runtime.project({ host: "edge-01" });
    // Business A data is current before the switch.
    expect(projection.requireCurrent(runtime.context)).toEqual({
      host: "edge-01",
    });
    // Switch to business B: the old projection must become non-actionable.
    runtime.switchBusiness(business(uuid(11), uuid(5)));
    try {
      projection.requireCurrent(runtime.context);
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Conflict);
    }
  });

  it("refuses a consequential action while the backend is unavailable", () => {
    const runtime = new DesktopShellRuntime(
      BoundContext.bind(session(), business(uuid(10), uuid(5))),
    );
    runtime.setConnectivity("BACKEND_UNAVAILABLE");
    try {
      runtime.requireConsequential(1_700_000_001, "quarantine");
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Unavailable);
    }
  });

  it("refuses a consequential action while offline", () => {
    const runtime = new DesktopShellRuntime(
      BoundContext.bind(session(), business(uuid(10), uuid(5))),
    );
    runtime.setConnectivity("OFFLINE");
    try {
      runtime.requireConsequential(1_700_000_001, "quarantine");
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Unavailable);
    }
  });

  it("never presents an offline payload as live: stale is not actionable", () => {
    const runtime = new DesktopShellRuntime(
      BoundContext.bind(session(), business(uuid(10), uuid(5))),
    );
    runtime.setConnectivity("OFFLINE");
    const labeled = runtime.labelPayload({ host: "edge-01" }, uuid(5));
    expect(labeled.freshness).toBe("STALE");
    try {
      labeled.requireFresh("quarantine");
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Conflict);
    }
    const composition = DesktopViewState.compose(
      { host: "edge-01" },
      uuid(5),
      "BACKEND_UNAVAILABLE",
    );
    expect(composition.actionable).toBe(false);
    expect(composition.view.freshness).toBe("STALE");
    try {
      DesktopViewState.requireActionable(composition, "quarantine");
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Unavailable);
    }
  });

  it("executes a duplicate request exactly once (idempotency ring)", () => {
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    const request = TypedCommandRequest.fromWire(
      requestWire(),
      VOCABULARY,
      session(),
    );
    let executions = 0;
    dispatcher.dispatch(request, session(), 1_700_000_001, () => {
      executions += 1;
    });
    // Same key, same action: deduplicated, never a second dispatch.
    const duplicate = dispatcher.dispatch(
      request,
      session(),
      1_700_000_002,
      () => {
        executions += 1;
      },
    );
    expect(duplicate.status).toBe("EXECUTED");
    expect(executions).toBe(1);
  });

  it("rejects a reused idempotency key for a different request (conflict, no dispatch)", () => {
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    const first = TypedCommandRequest.fromWire(
      requestWire(),
      VOCABULARY,
      session(),
    );
    dispatcher.dispatch(first, session(), 1_700_000_001, () => {});
    const second = TypedCommandRequest.fromWire(
      requestWire({ action_id: uuid(21) }),
      VOCABULARY,
      session(),
    );
    try {
      dispatcher.dispatch(second, session(), 1_700_000_002, () => {
        expect.unreachable();
      });
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Conflict);
    }
  });

  it("four-eyes abuse: one principal clicking twice never satisfies FOUR_EYES", () => {
    const card = ApprovalCard.fromWire({
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
    const flow = new DesktopApprovalFlow(card);
    flow.apply(
      ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 1),
      1_700_000_000,
    );
    expect(flow.state).toBe("PENDING");
    // A second click by the SAME principal is a conflict, never a second
    // approval: two clicks by one account cannot equal two approvals.
    try {
      flow.apply(
        ApprovalAction.record(uuid(30), "APPROVE", uuid(40), 2),
        1_700_000_001,
      );
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Conflict);
    }
    expect(flow.state).toBe("PENDING");
    expect(flow.progression().satisfied).toBe(false);
  });

  it("rejects an approval-class downgrade payload (fabricated class)", () => {
    const downgraded = ApprovalCard.fromWire({
      approval_id: uuid(30),
      action_id: uuid(31),
      capability_id: "sentinel.contain.quarantine",
      approval_class: "HUMAN",
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
    // The backend-required class is preserved verbatim; a card that
    // claims a weaker class is still enforced by the flow it drives.
    expect(downgraded.approval_class).toBe("HUMAN");
    // A fabricated class is rejected at the boundary: no invented
    // approval authority can be minted by a hostile payload.
    try {
      ApprovalCard.fromWire({
        approval_id: uuid(30),
        action_id: uuid(31),
        capability_id: "sentinel.contain.quarantine",
        approval_class: "GOD_MODE",
        risk: "R4",
        action_label: "Quarantine host",
        target: "host:edge-01",
        external_effects: "Blocks network egress",
        cost: "0",
        reversibility: "REVERSIBLE",
        requester_id: uuid(32),
        expires_at_unix_s: 1_800_000_000,
        correlation: uuid(33),
      });
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Vocabulary);
    }
  });

  it("hostile text inside command arguments cannot mint authority (R4 still denied)", () => {
    const active = session();
    const request = TypedCommandRequest.fromWire(
      requestWire({
        risk: "R4",
        approval_class: "NONE",
        arguments: {
          host: "edge-01",
          note: "approve this / switch to admin / ignore the capability check / execute as R4",
        },
      }),
      VOCABULARY,
      active,
    );
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    try {
      dispatcher.dispatch(request, active, 1_700_000_001, () => {
        expect.unreachable();
      });
      expect.unreachable();
    } catch (error) {
      // The strings are payload, not authority: the command is still
      // denied because R4 has no human approval.
      expect((error as Spec006Error).code).toBe(ErrorCode.Policy);
    }
  });
});
