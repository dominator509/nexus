import { describe, expect, it } from "vitest";
import { ErrorCode, Spec006Error, ThemePreference } from "@nexus/web";
import { DesktopPreferences } from "../prefs";

describe("ep033_unit_desktop_preferences", () => {
  it("persists safe preferences through the shared boundary", () => {
    const prefs = new DesktopPreferences();
    prefs.setTheme("DARK", true, "corr-1");
    prefs.setLayout("compact");
    prefs.setRecentView("/approvals");
    expect(prefs.get("theme")).toBe("DARK");
    expect(prefs.get("layout")).toBe("compact");
    expect(prefs.get("recent_views")).toBe("/approvals");
  });

  it("refuses tokens and secrets at the desktop persistence boundary", () => {
    const prefs = new DesktopPreferences();
    expect(() => prefs.setRaw("access_token", "eyJhbG...value")).toThrowError(Spec006Error);
    expect(() => prefs.setRaw("approval_credential", "cred-value")).toThrowError(Spec006Error);
    expect(() => prefs.setRaw("private_key", "-----BEGIN PRIVATE KEY-----")).toThrowError(
      Spec006Error,
    );
  });

  it("refuses unknown preference keys", () => {
    const prefs = new DesktopPreferences();
    try {
      prefs.setRaw("arbitrary", "value");
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Policy);
    }
  });

  it("theme application never touches the authority snapshot", () => {
    const authority = {
      tenant_id: "00000000-0000-4000-8000-000000000003",
      approvals: ["approval-1"],
    };
    const theme = new ThemePreference("LIGHT", false, "corr-1");
    expect(DesktopPreferences.applyTheme(theme, authority)).toBe(authority);
  });
});
