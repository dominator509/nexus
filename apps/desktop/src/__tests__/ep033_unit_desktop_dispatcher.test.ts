import { describe, expect, it } from "vitest";
import {
  AuthenticatedSession,
  ErrorCode,
  KnownCapabilityVocabulary,
  Spec006Error,
  TypedCommandRequest,
} from "@nexus/web";
import { DesktopCommandDispatcher, DesktopTelemetry } from "../dispatcher";

function uuid(n: number): string {
  return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
}

function session(expiresAt = 1_800_000_000): AuthenticatedSession {
  return AuthenticatedSession.fromWire({
    session_id: uuid(1),
    principal_id: uuid(2),
    tenant_id: uuid(3),
    device_id: uuid(4),
    grant_flow: "AUTHORIZATION_CODE",
    strength: "MULTI_FACTOR",
    created_at_unix_s: 1_700_000_000,
    expires_at_unix_s: expiresAt,
    revoked: false,
    correlation: uuid(5),
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
      origin_system: "desktop",
      external_actor_id: uuid(2),
      external_actor_type: "principal",
    },
    ...overrides,
  };
}

const VOCABULARY = new KnownCapabilityVocabulary([
  "home.lights.query",
  "home.lights.set",
  "sentinel.contain.quarantine",
]);

describe("ep033_unit_desktop_dispatcher", () => {
  it("dispatches a validated command exactly once and returns EXECUTED", () => {
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    const request = TypedCommandRequest.fromWire(
      requestWire(),
      VOCABULARY,
      session(),
    );
    let executions = 0;
    const result = dispatcher.dispatch(
      request,
      session(),
      1_700_000_001,
      () => {
        executions += 1;
      },
    );
    expect(result.status).toBe("EXECUTED");
    expect(result.capability_id).toBe("home.lights.set");
    expect(executions).toBe(1);
  });

  it("refuses dispatch under an expired session (fail closed)", () => {
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    const expired = session(1_000_000_000);
    const request = TypedCommandRequest.fromWire(
      requestWire(),
      VOCABULARY,
      expired,
    );
    expect(() =>
      dispatcher.dispatch(request, expired, 1_700_000_001, () => {
        expect.unreachable();
      }),
    ).toThrowError(Spec006Error);
  });

  it("never double-executes a duplicate idempotency key", () => {
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    const active = session();
    const request = TypedCommandRequest.fromWire(
      requestWire(),
      VOCABULARY,
      active,
    );
    let executions = 0;
    dispatcher.dispatch(request, active, 1_700_000_001, () => {
      executions += 1;
    });
    const duplicate = TypedCommandRequest.fromWire(
      requestWire(),
      VOCABULARY,
      active,
    );
    const result = dispatcher.dispatch(duplicate, active, 1_700_000_001, () => {
      executions += 1;
    });
    expect(executions).toBe(1);
    expect(result.status).toBe("EXECUTED");
  });

  it("rejects an idempotency key reused for a different request", () => {
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    const active = session();
    const first = TypedCommandRequest.fromWire(
      requestWire(),
      VOCABULARY,
      active,
    );
    dispatcher.dispatch(first, active, 1_700_000_001, () => {});
    const different = TypedCommandRequest.fromWire(
      requestWire({ action_id: uuid(21) }),
      VOCABULARY,
      active,
    );
    try {
      dispatcher.dispatch(different, active, 1_700_000_001, () => {});
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Conflict);
    }
  });

  it("fails closed on R3/R4 commands without human approval", () => {
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    const active = session();
    const risky = TypedCommandRequest.fromWire(
      requestWire({
        capability_id: "sentinel.contain.quarantine",
        risk: "R3",
        approval_class: "NONE",
      }),
      VOCABULARY,
      active,
    );
    try {
      dispatcher.dispatch(risky, active, 1_700_000_001, () => {
        expect.unreachable();
      });
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Policy);
    }
  });

  it("permits R3/R4 commands with HUMAN approval class", () => {
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    const active = session();
    const approved = TypedCommandRequest.fromWire(
      requestWire({
        capability_id: "sentinel.contain.quarantine",
        risk: "R3",
        approval_class: "HUMAN",
      }),
      VOCABULARY,
      active,
    );
    let executed = false;
    dispatcher.dispatch(approved, active, 1_700_000_001, () => {
      executed = true;
    });
    expect(executed).toBe(true);
  });

  it("propagates execute failures as errors, never as success", () => {
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    const active = session();
    const request = TypedCommandRequest.fromWire(
      requestWire(),
      VOCABULARY,
      active,
    );
    expect(() =>
      dispatcher.dispatch(request, active, 1_700_000_001, () => {
        throw new Spec006Error(ErrorCode.Unavailable, "backend refused");
      }),
    ).toThrowError(Spec006Error);
  });

  it("rejects unknown capability ids before dispatch (no fabricated capabilities)", () => {
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    const active = session();
    expect(() =>
      TypedCommandRequest.fromWire(
        requestWire({ capability_id: "invented.panel.action" }),
        VOCABULARY,
        active,
      ),
    ).toThrowError(Spec006Error);
  });
});

describe("ep033_unit_desktop_telemetry", () => {
  it("records only safe fields and redacts secret-shaped content", () => {
    const telemetry = new DesktopTelemetry();
    telemetry.record({
      action: "dispatch",
      capability_id: "home.lights.set",
      correlation_id: uuid(5),
      outcome: "EXECUTED",
      duration_ms: 3,
    });
    telemetry.record({
      action: "dispatch",
      capability_id: "token=abcdefghijklmnopqrstuvwxyz",
      correlation_id: "corr-2",
      outcome: "EXECUTED",
      duration_ms: 4,
    });
    telemetry.assertNoSecrets();
    const serialized = JSON.stringify(telemetry.entries());
    expect(serialized).not.toMatch(/token=/);
    expect(serialized).toContain("[REDACTED]");
  });

  it("canary fails when a secret-shaped value would survive", () => {
    const telemetry = new DesktopTelemetry();
    telemetry.record({
      action: "Authorization: Bearer abcdefghijklmnopqrstuvwxyz",
      capability_id: "home.lights.set",
      correlation_id: "corr-3",
      outcome: "REJECTED",
      duration_ms: 1,
    });
    expect(() => telemetry.assertNoSecrets()).toThrowError(Spec006Error);
  });
});
