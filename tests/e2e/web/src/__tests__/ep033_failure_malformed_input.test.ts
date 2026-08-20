/**
 * EP-033 M4 forced failures: corrupted wire input (directive E/K).
 *
 * The failure mechanism is a CORRUPTED CONTROLLED MESSAGE: malformed,
 * unknown-field, and fabricated-vocabulary inputs are pushed through
 * the real contract validators. Nothing is mocked; the contracts
 * themselves are the component under test.
 */

import { describe, expect, it } from "vitest";
import {
  AuthenticatedSession,
  ErrorCode,
  PresentedCapability,
  Spec006Error,
  TypedCommandRequest,
  KnownCapabilityVocabulary,
} from "@nexus/web";

function uuid(n: number): string {
  return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
}

function session(): AuthenticatedSession {
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

const VOCABULARY = new KnownCapabilityVocabulary(["home.lights.query", "home.lights.set"]);

describe("ep033_failure_malformed_input", () => {
  it("rejects a session with unknown fields (deny-unknown fail closed)", () => {
    const corrupted = {
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
      superuser: true,
    };
    expect(() => AuthenticatedSession.fromWire(corrupted)).toThrowError(Spec006Error);
  });

  it("rejects a session with a corrupted enum", () => {
    const corrupted = {
      session_id: uuid(1),
      principal_id: uuid(2),
      tenant_id: uuid(3),
      device_id: uuid(4),
      grant_flow: "MAGIC",
      strength: "MULTI_FACTOR",
      created_at_unix_s: 1_700_000_000,
      expires_at_unix_s: 1_800_000_000,
      revoked: false,
      correlation: uuid(5),
    };
    try {
      AuthenticatedSession.fromWire(corrupted);
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Vocabulary);
    }
  });

  it("rejects a corrupted capability descriptor with a fabricated class", () => {
    expect(() =>
      PresentedCapability.fromWire({
        capability_id: "home.lights.set",
        class: "GOD_MODE",
        availability: "AVAILABLE",
        visible: true,
        authorized: true,
        required_approval: "NONE",
      }),
    ).toThrowError(Spec006Error);
  });

  it("rejects a command with a fabricated capability id before dispatch", () => {
    const active = session();
    const fabricated = {
      action_id: uuid(20),
      tenant_id: uuid(3),
      principal_id: uuid(2),
      capability_id: "invented.panel.action",
      idempotency_key: "req-000000000000001",
      risk: "R0",
      approval_class: "NONE",
      reversal: "none",
      arguments: {},
      expected_state: {},
      invocation: {
        request_id: "req-000000000000002",
        correlation_id: uuid(5),
        origin_system: "web",
        external_actor_id: uuid(2),
        external_actor_type: "principal",
      },
    };
    try {
      TypedCommandRequest.fromWire(fabricated, VOCABULARY, active);
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Vocabulary);
    }
  });

  it("rejects a command with an undersized idempotency key", () => {
    const active = session();
    const malformed = {
      action_id: uuid(20),
      tenant_id: uuid(3),
      principal_id: uuid(2),
      capability_id: "home.lights.set",
      idempotency_key: "short",
      risk: "R1",
      approval_class: "NONE",
      reversal: "home.lights.set:reverse",
      arguments: {},
      expected_state: {},
      invocation: {
        request_id: "req-000000000000002",
        correlation_id: uuid(5),
        origin_system: "web",
        external_actor_id: uuid(2),
        external_actor_type: "principal",
      },
    };
    expect(() => TypedCommandRequest.fromWire(malformed, VOCABULARY, active)).toThrowError(
      Spec006Error,
    );
  });
});
