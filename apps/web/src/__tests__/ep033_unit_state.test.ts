import { describe, expect, it } from "vitest";
import { ViewState, revalidated, UI_STATE_KINDS, CONNECTIVITY_STATES } from "../contracts/state";
import { ErrorCode, Spec006Error } from "../contracts/errors";

describe("ep033_unit_state_vocabulary", () => {
  it("covers every SPEC-004 UI state kind", () => {
    expect([...UI_STATE_KINDS].sort()).toEqual(
      ["LOADING", "EMPTY", "ERROR", "DEGRADED", "PERMISSION_DENIED", "SUCCESS"].sort(),
    );
  });

  it("distinguishes every connectivity state explicitly", () => {
    expect([...CONNECTIVITY_STATES]).toEqual([
      "CONNECTED",
      "DEGRADED",
      "OFFLINE",
      "AUTH_EXPIRED",
      "BACKEND_UNAVAILABLE",
    ]);
  });

  it("labels fresh and stale data distinctly", () => {
    const fresh = ViewState.success({ value: 1 }, "corr-1");
    expect(fresh.freshness).toBe("FRESH");
    const stale = ViewState.degraded("corr-2", "OFFLINE", 1);
    expect(stale.freshness).toBe("STALE");
  });

  it("refuses consequential action on stale data", () => {
    const stale = ViewState.degraded("corr-2", "OFFLINE", 1);
    try {
      stale.requireFresh("mutate");
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Conflict);
    }
  });

  it("refuses consequential action on empty/error payloads", () => {
    const empty = ViewState.empty("corr-3");
    expect(() => empty.requireFresh("mutate")).toThrowError(Spec006Error);
  });

  it("permits consequential action on fresh verified payload", () => {
    const fresh = ViewState.success({ value: 1 }, "corr-4", { revision: 7 });
    expect(fresh.requireFresh("mutate")).toEqual({ value: 1 });
  });

  it("rejects revalidation to an older revision", () => {
    const current = ViewState.success({ value: 2 }, "corr-5", { revision: 9 });
    const older = ViewState.success({ value: 1 }, "corr-6", { revision: 3 });
    expect(() => revalidated(current, older)).toThrowError(Spec006Error);
  });

  it("accepts revalidation to a newer revision", () => {
    const current = ViewState.success({ value: 1 }, "corr-7", { revision: 3 });
    const newer = ViewState.success({ value: 2 }, "corr-8", { revision: 4 });
    expect(revalidated(current, newer).payload).toEqual({ value: 2 });
  });

  it("maps permission-denied errors to the permission-denied state", () => {
    const denied = ViewState.error(
      new Spec006Error(ErrorCode.Policy, "denied"),
      "corr-9",
    );
    expect(denied.kind).toBe("PERMISSION_DENIED");
  });

  it("maps generic errors to the error state, never success", () => {
    const failed = ViewState.error(
      new Spec006Error(ErrorCode.Unavailable, "backend down"),
      "corr-10",
    );
    expect(failed.kind).toBe("ERROR");
  });
});
