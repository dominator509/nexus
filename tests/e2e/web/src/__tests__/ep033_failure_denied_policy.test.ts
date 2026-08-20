/**
 * EP-033 M4 forced failures: denied policy and authorization.
 *
 * The failure mechanism is a DENIED POLICY DECISION: commands whose
 * risk or session state forbids execution are pushed through the real
 * desktop dispatcher and the real contract gate. Denials are terminal;
 * nothing is queued for blind replay (directive N).
 */

import { describe, expect, it } from "vitest";
import {
  AuthenticatedSession,
  ErrorCode,
  KnownCapabilityVocabulary,
  Spec006Error,
  TypedCommandRequest,
} from "@nexus/web";
import { DesktopCommandDispatcher } from "@nexus/desktop";

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

function requestWire(
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  return {
    action_id: uuid(20),
    tenant_id: uuid(3),
    principal_id: uuid(2),
    capability_id: "sentinel.contain.quarantine",
    idempotency_key: "req-000000000000001",
    risk: "R3",
    approval_class: "HUMAN",
    reversal: "sentinel.contain.release",
    arguments: { host: "edge-01" },
    expected_state: { quarantined: true },
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

const VOCABULARY = new KnownCapabilityVocabulary([
  "home.lights.query",
  "home.lights.set",
  "sentinel.contain.quarantine",
]);

describe("ep033_failure_denied_policy", () => {
  it("refuses an R3 command without human approval (policy denied)", () => {
    const active = session();
    const request = TypedCommandRequest.fromWire(
      requestWire({ approval_class: "NONE" }),
      VOCABULARY,
      active,
    );
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    expect(() =>
      dispatcher.dispatch(request, active, 1_700_000_001, () => {
        expect.unreachable();
      }),
    ).toThrowError(Spec006Error);
  });

  it("refuses an R4 command under POLICY approval (never auto-executes)", () => {
    const active = session();
    const request = TypedCommandRequest.fromWire(
      requestWire({ risk: "R4", approval_class: "POLICY" }),
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
      expect((error as Spec006Error).code).toBe(ErrorCode.Policy);
    }
  });

  it("refuses dispatch under a revoked session (authorization denied)", () => {
    const revoked = session(1_800_000_000, true);
    const request = TypedCommandRequest.fromWire(
      requestWire(),
      VOCABULARY,
      revoked,
    );
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    try {
      dispatcher.dispatch(request, revoked, 1_700_000_001, () => {
        expect.unreachable();
      });
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Authorization);
    }
  });

  it("refuses dispatch under an expired session and never replays", () => {
    const expired = session(1_000_000_000);
    const request = TypedCommandRequest.fromWire(
      requestWire(),
      VOCABULARY,
      expired,
    );
    const dispatcher = new DesktopCommandDispatcher(VOCABULARY);
    // First attempt: refused.
    expect(() =>
      dispatcher.dispatch(request, expired, 1_700_000_001, () => {}),
    ).toThrowError(Spec006Error);
    // Second attempt without re-authentication: refused again - no
    // blind replay queue exists.
    expect(() =>
      dispatcher.dispatch(request, expired, 1_700_000_002, () => {}),
    ).toThrowError(Spec006Error);
  });
});
