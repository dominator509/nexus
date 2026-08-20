import { describe, expect, it } from "vitest";
import {
  PreferencePersistence,
  ThemePreference,
  THEME_MODES,
  ALLOWED_PREFERENCE_KEYS,
  FORBIDDEN_PREFERENCE_KEYS,
} from "../contracts/preferences";
import { ErrorCode, Spec006Error } from "../contracts/errors";

describe("ep033_unit_preferences", () => {
  it("persists only allowlisted non-sensitive preferences", () => {
    const store = new PreferencePersistence();
    store.set("theme", "dark");
    store.set("layout", "compact");
    store.set("reduced_motion", "true");
    expect(store.get("theme")).toBe("dark");
  });

  it("exposes the canonical theme modes", () => {
    expect([...THEME_MODES]).toEqual(["LIGHT", "DARK", "SYSTEM"]);
  });

  it("refuses unknown preference keys", () => {
    const store = new PreferencePersistence();
    try {
      store.set("arbitrary_key", "value");
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Policy);
    }
  });

  it("refuses bearer tokens and secrets at the persistence boundary", () => {
    const store = new PreferencePersistence();
    expect(() => store.set("access_token", "eyJhbGciOiJIUzI1NiJ9.xxxx.yyyy")).toThrowError(
      Spec006Error,
    );
    expect(() => store.set("refresh_token", "rt-1234567890")).toThrowError(Spec006Error);
    expect(() => store.set("approval_credential", "cred-12345678")).toThrowError(Spec006Error);
    expect(() => store.set("private_key", "-----BEGIN FIXTURE KEY-----")).toThrowError(
      Spec006Error,
    );
    expect(() => store.set("recovery_kit", "recovery-12345678")).toThrowError(Spec006Error);
  });

  it("refuses secret-shaped values even under allowlisted keys", () => {
    const store = new PreferencePersistence();
    // Defense in depth: a token-shaped value must not be stored even
    // under an innocent key.
    expect(() => store.set("layout", "Bearer eyJhbGciOiJIUzI1NiJ9.xxxx.yyyy")).toThrowError(
      Spec006Error,
    );
    expect(() => store.set("theme", "password=hunter2")).toThrowError(Spec006Error);
  });

  it("exposes the forbidden key allowlist for canary auditing", () => {
    expect(FORBIDDEN_PREFERENCE_KEYS.has("access_token")).toBe(true);
    expect(FORBIDDEN_PREFERENCE_KEYS.has("refresh_token")).toBe(true);
    expect(FORBIDDEN_PREFERENCE_KEYS.has("secret")).toBe(true);
    expect(FORBIDDEN_PREFERENCE_KEYS.has("password")).toBe(true);
    expect(FORBIDDEN_PREFERENCE_KEYS.has("approval_credential")).toBe(true);
  });

  it("theme change never mutates authority state", () => {
    const authority = {
      tenant_id: "00000000-0000-4000-8000-000000000003",
      capabilities: new Set(["home.lights.query"]),
      approvals: ["approval-0001"],
    };
    const theme = new ThemePreference("DARK", true, "corr-0001");
    const after = ThemePreference.apply(theme, authority);
    // The theme is rendered, not applied: authority state is untouched.
    expect(after).toBe(authority);
  });

  it("rejects unsupported theme modes", () => {
    expect(() => new ThemePreference("NEON" as never, false, "corr-1")).toThrowError(Spec006Error);
  });

  it("keeps preference state structurally separate from security state", () => {
    const store = new PreferencePersistence();
    store.set("theme", "light");
    const snapshot = store.snapshot();
    expect(snapshot.has("theme")).toBe(true);
    // A preference store has no tenant, permission, or approval keys.
    for (const key of snapshot.keys()) {
      expect(key.startsWith("tenant_")).toBe(false);
      expect(key.startsWith("permission")).toBe(false);
      expect(key.startsWith("approval_")).toBe(false);
    }
  });

  it("allowlist is exactly the declared safe set", () => {
    expect([...ALLOWED_PREFERENCE_KEYS].sort()).toEqual(
      ["theme", "layout", "reduced_motion", "recent_views"].sort(),
    );
  });
});
