import { describe, expect, it } from "vitest";
import {
  APPROVAL_CLASSES,
  DispatchGate,
  TypedCommandRequest,
  refuseOrPass,
  lifecycleClaims,
  RISK_CLASSES,
} from "../contracts/command";
import { AuthenticatedSession } from "../contracts/session";
import { KnownCapabilityVocabulary } from "../contracts/capability";
import { ErrorCode, Spec006Error } from "../contracts/errors";

function uuid(n: number): string {
  return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
}

function session(expiresAt = 1_800_000_000, revoked = false): AuthenticatedSession {
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

function requestWire(overrides: Record<string, unknown> = {}): Record<string, unknown> {
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

const VOCABULARY = new KnownCapabilityVocabulary(["home.lights.query", "home.lights.set"]);

describe("ep033_unit_command_typed_dispatch", () => {
  it("constructs a valid typed command from canonical ActionRequest shape", () => {
    const request = TypedCommandRequest.fromWire(requestWire(), VOCABULARY, session());
    expect(request.capability_id).toBe("home.lights.set");
    expect(request.risk).toBe("R1");
    expect(request.invocation.correlation_id).toBe(uuid(5));
  });

  it("rejects unknown capability ids (no fabricated capability names)", () => {
    try {
      TypedCommandRequest.fromWire(
        requestWire({ capability_id: "home.lights.hack" }),
        VOCABULARY,
        session(),
      );
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Vocabulary);
    }
  });

  it("rejects capability ids outside the vocabulary even when risk is low", () => {
    expect(() =>
      TypedCommandRequest.fromWire(
        requestWire({ capability_id: "invented.panel.action" }),
        VOCABULARY,
        session(),
      ),
    ).toThrowError(Spec006Error);
  });

  it("rejects commands whose tenant/principal do not match the session", () => {
    expect(() =>
      TypedCommandRequest.fromWire(
        requestWire({ tenant_id: uuid(99) }),
        VOCABULARY,
        session(),
      ),
    ).toThrowError(Spec006Error);
  });

  it("rejects invalid idempotency keys", () => {
    expect(() =>
      TypedCommandRequest.fromWire(
        requestWire({ idempotency_key: "short" }),
        VOCABULARY,
        session(),
      ),
    ).toThrowError(Spec006Error);
  });

  it("rejects unsupported risk and approval classes", () => {
    expect(() =>
      TypedCommandRequest.fromWire(requestWire({ risk: "R9" }), VOCABULARY, session()),
    ).toThrowError(Spec006Error);
    expect(() =>
      TypedCommandRequest.fromWire(
        requestWire({ approval_class: "MAYBE" }),
        VOCABULARY,
        session(),
      ),
    ).toThrowError(Spec006Error);
  });

  it("rejects unknown fields (deny-unknown)", () => {
    expect(() =>
      TypedCommandRequest.fromWire(
        requestWire({ action_name: "set-lights" }),
        VOCABULARY,
        session(),
      ),
    ).toThrowError(Spec006Error);
  });

  it("rejects incomplete invocation context", () => {
    const wire = requestWire();
    (wire.invocation as Record<string, unknown>).request_id = undefined;
    expect(() => TypedCommandRequest.fromWire(wire, VOCABULARY, session())).toThrowError(
      Spec006Error,
    );
  });

  it("serializes back to the canonical ActionRequest wire shape", () => {
    const request = TypedCommandRequest.fromWire(requestWire(), VOCABULARY, session());
    const wire = request.toWire();
    expect(wire.action_id).toBe(uuid(20));
    expect(wire.capability_id).toBe("home.lights.set");
    expect(wire.invocation.origin_system).toBe("web");
  });

  it("gate refuses dispatch under an expired session (fail closed)", () => {
    const expired = session(1_000_000_000);
    const request = TypedCommandRequest.fromWire(requestWire(), VOCABULARY, expired);
    const gate = new DispatchGate();
    expect(() => gate.authorize(request, expired, 1_700_000_001)).toThrowError(Spec006Error);
  });

  it("expiry refusal is terminal: never queued for blind replay", () => {
    const expired = session(1_000_000_000);
    const request = TypedCommandRequest.fromWire(requestWire(), VOCABULARY, expired);
    const outcome = refuseOrPass(request, expired, 1_700_000_001);
    expect(outcome.refusal.code).toBe("AUTH_EXPIRED");
    // The contract's terminal semantics: the caller re-authenticates
    // and re-issues a fresh request; no replay queue exists.
    expect(() => DispatchGate.prototype.authorize.call(gate(), request, expired, 1_700_000_001)).toThrowError(
      Spec006Error,
    );
    function gate(): DispatchGate {
      return new DispatchGate();
    }
  });

  it("gate permits dispatch under an active session", () => {
    const active = session();
    const request = TypedCommandRequest.fromWire(requestWire(), VOCABULARY, active);
    const gate = new DispatchGate();
    expect(gate.authorize(request, active, 1_700_000_001)).toBe(request);
  });

  it("exposes the full canonical approval and risk vocabulary", () => {
    expect([...APPROVAL_CLASSES]).toEqual([
      "NONE",
      "POLICY",
      "HUMAN",
      "STRONG_HUMAN",
      "FOUR_EYES",
    ]);
    expect([...RISK_CLASSES]).toEqual(["R0", "R1", "R2", "R3", "R4"]);
  });

  it("lifecycle claims never assert executed/verified for a mere request", () => {
    expect(lifecycleClaims("REQUESTED")).toEqual({ requested: true, executed: false, verified: false });
    expect(lifecycleClaims("SUCCEEDED").executed).toBe(true);
    expect(lifecycleClaims("SUCCEEDED").verified).toBe(true);
    expect(lifecycleClaims("AWAITING_APPROVAL").verified).toBe(false);
  });
});
