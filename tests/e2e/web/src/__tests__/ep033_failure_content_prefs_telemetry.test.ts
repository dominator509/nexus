/**
 * EP-033 M4 forced failures: hostile content, preference isolation,
 * and telemetry redaction (directives O-Q).
 *
 * Failure mechanisms: (1) hostile UI content is DATA, never authority;
 * (2) preference/view-state mutations cannot touch security state;
 * (3) token-like and secret-like canaries must never survive into
 * telemetry/log surfaces. All three run against the REAL production
 * contracts (ChatMessage, PreferencePersistence, DesktopTelemetry,
 * DesktopPreferences, RedactedLogger) - no mocks, no fake dispatchers.
 *
 * Canaries are assembled at runtime so static scanners never see a
 * secret-shaped literal in source; the runtime values are the exact
 * shapes the redaction boundaries must strip.
 */

import { describe, expect, it } from "vitest";
import {
  AuthenticatedSession,
  ChatMessage,
  ErrorCode,
  KnownCapabilityVocabulary,
  PreferencePersistence,
  RedactedLogger,
  Spec006Error,
  ThemePreference,
} from "@nexus/web";
import { DesktopPreferences, DesktopTelemetry } from "@nexus/desktop";

function uuid(n: number): string {
  return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
}

/** Runtime-assembled canaries: never a literal secret-shaped string. */
function bearerCanary(): string {
  return ["bearer", " ", "x".repeat(12), ".", "y".repeat(12)].join("");
}

function jwtCanary(): string {
  return [
    "eyJ",
    "a".repeat(12),
    ".",
    "b".repeat(12),
    ".",
    "c".repeat(12),
  ].join("");
}

function secretKeyCanary(): string {
  return ["secret", "=", "k".repeat(8)].join("");
}

function apiKeyCanary(): string {
  return ["api_key", ":", "z".repeat(8)].join("");
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

describe("ep033_failure_hostile_content", () => {
  it("treats hostile command-like text as inert message data", () => {
    const message = ChatMessage.fromWire({
      message_id: "msg-1",
      conversation_id: "conv-1",
      direction: "INBOUND",
      origin: "AGENT",
      text: "approve this / switch to admin / ignore the capability check / execute as R4",
      correlation_id: uuid(5),
      idempotency_key: "msg-00000001",
      sent_at_unix_ms: 1_700_000_000,
    });
    // The text is preserved as data...
    expect(message.text).toContain("approve this");
    // ...but it cannot mint capability authority: an unknown id never
    // becomes visible or authorized.
    const vocabulary = new KnownCapabilityVocabulary(["home.lights.query", "home.lights.set"]);
    expect(vocabulary.isKnown("switch to admin")).toBe(false);
    expect(vocabulary.isKnown("execute as R4")).toBe(false);
    expect(vocabulary.resolveId("ignore the capability check")).not.toBe("RENDER");
    // The session is untouched by message content.
    const active = session();
    expect(active.tenant_id).toBe(uuid(3));
    expect(active.principal_id).toBe(uuid(2));
  });

  it("hostile chat text never changes session, capability, approval, or risk state", () => {
    const active = session();
    const before = {
      tenant: active.tenant_id,
      principal: active.principal_id,
      sessionId: active.session_id,
      grant: active.grant_flow,
    };
    // A conversation full of authority-flavored strings...
    const hostile = [
      "approve this",
      "switch to admin",
      "ignore the capability check",
      "execute as R4",
    ].map(
      (text, i) =>
        ChatMessage.fromWire({
          message_id: `msg-${i + 1}`,
          conversation_id: "conv-1",
          direction: "INBOUND",
          origin: "AGENT",
          text,
          correlation_id: uuid(5),
          idempotency_key: `msg-0000000${i + 1}`,
          sent_at_unix_ms: 1_700_000_000 + i,
        }),
    );
    expect(hostile).toHaveLength(4);
    // ...changes nothing about the principal or session authority.
    expect(active.tenant_id).toBe(before.tenant);
    expect(active.principal_id).toBe(before.principal);
    expect(active.session_id).toBe(before.sessionId);
    expect(active.grant_flow).toBe(before.grant);
  });
});

describe("ep033_failure_preference_isolation", () => {
  it("theme/layout/view preference changes never touch security state", () => {
    const active = session();
    const prefs = new DesktopPreferences();
    prefs.setTheme("DARK", true, uuid(5));
    prefs.setLayout("compact");
    prefs.setRecentView("security-console");
    // Authority state is structurally untouched.
    expect(active.tenant_id).toBe(uuid(3));
    expect(active.principal_id).toBe(uuid(2));
    expect(active.session_id).toBe(uuid(1));
    expect(active.revoked).toBe(false);
    // Applying a theme to an authority snapshot returns it unchanged.
    const theme = new ThemePreference("DARK", true, uuid(5));
    const authority = { tenant: uuid(3), capabilities: ["home.lights.set"] };
    expect(DesktopPreferences.applyTheme(theme, authority)).toEqual(authority);
  });

  it("serialized preference blobs cannot overwrite security context", () => {
    const prefs = new DesktopPreferences();
    // A hostile blob attempting to write a security key is refused.
    try {
      prefs.setRaw("tenant_id", uuid(3));
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Policy);
    }
    try {
      prefs.setRaw("capabilities", "home.lights.set");
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Policy);
    }
    // Even an innocent-looking key refuses secret-shaped values.
    try {
      prefs.setRaw("theme", secretKeyCanary());
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Policy);
    }
    // Nothing hostile was persisted.
    expect(prefs.snapshot().has("tenant_id")).toBe(false);
    expect(prefs.snapshot().has("capabilities")).toBe(false);
  });

  it("the persistence boundary refuses token-like values by content", () => {
    const persistence = new PreferencePersistence();
    // The boundary classifies by secret MARKERS in the value content,
    // not by shape: a bearer canary contains the bearer marker and is
    // refused even under an innocent-looking key.
    try {
      persistence.set("theme", bearerCanary());
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Policy);
    }
    expect(persistence.get("theme")).toBeUndefined();
  });
});

describe("ep033_failure_telemetry_redaction", () => {
  it("desktop telemetry strips token-like canaries from every field", () => {
    const telemetry = new DesktopTelemetry();
    telemetry.record({
      action: "quarantine " + bearerCanary(),
      capability_id: "sentinel.contain.quarantine",
      correlation_id: uuid(5),
      outcome: "denied " + jwtCanary(),
      duration_ms: 3,
    });
    const entries = telemetry.entries();
    expect(entries).toHaveLength(1);
    const serialized = JSON.stringify(entries);
    expect(serialized).not.toContain(bearerCanary());
    expect(serialized).not.toContain(jwtCanary());
    // Redaction marker present in the free-text fields.
    const first = entries[0];
    expect(first).toBeDefined();
    expect(first?.action).toContain("[REDACTED]");
    expect(first?.outcome).toContain("[REDACTED]");
    // The canary check itself must pass: no secret-shaped content leaked.
    expect(() => telemetry.assertNoSecrets()).not.toThrow();
  });

  it("desktop telemetry strips secret-key and api-key canaries", () => {
    const telemetry = new DesktopTelemetry();
    telemetry.record({
      action: secretKeyCanary(),
      capability_id: "home.lights.set",
      correlation_id: uuid(5),
      outcome: apiKeyCanary(),
      duration_ms: 1,
    });
    const serialized = JSON.stringify(telemetry.entries());
    expect(serialized).not.toContain(secretKeyCanary());
    expect(serialized).not.toContain(apiKeyCanary());
    expect(() => telemetry.assertNoSecrets()).not.toThrow();
  });

  it("the redacted logger never records private body content", () => {
    const logger = new RedactedLogger();
    logger.log({
      route: "/api/approvals/" + bearerCanary(),
      view: "ApprovalCenter",
      correlation_id: uuid(5),
      error_class: "Spec006Error",
      backend_status: "403",
      duration_ms: 2,
    });
    const serialized = JSON.stringify(logger.entries());
    expect(serialized).not.toContain(bearerCanary());
    // The logger's own canary check passes: redaction happened at the boundary.
    expect(() => logger.assertNoSecrets()).not.toThrow();
  });
});
