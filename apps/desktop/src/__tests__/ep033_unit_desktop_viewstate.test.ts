import { describe, expect, it } from "vitest";
import { ErrorCode, Spec006Error, ViewState } from "@nexus/web";
import { DesktopViewState } from "../viewstate";

describe("ep033_unit_desktop_viewstate", () => {
  it("composes a connected payload as fresh and actionable", () => {
    const composition = DesktopViewState.compose(
      { value: 1 },
      "corr-1",
      "CONNECTED",
    );
    expect(composition.actionable).toBe(true);
    expect(composition.view.freshness).toBe("FRESH");
    expect(DesktopViewState.requireActionable(composition, "mutate")).toEqual({
      value: 1,
    });
  });

  it("labels offline payloads stale and refuses consequential action", () => {
    const composition = DesktopViewState.compose(
      { value: 1 },
      "corr-2",
      "OFFLINE",
    );
    expect(composition.actionable).toBe(false);
    expect(composition.view.freshness).toBe("STALE");
    try {
      DesktopViewState.requireActionable(composition, "mutate");
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Unavailable);
    }
  });

  it("labels backend-unavailable payloads stale", () => {
    const composition = DesktopViewState.compose(
      { value: 1 },
      "corr-3",
      "BACKEND_UNAVAILABLE",
    );
    expect(composition.actionable).toBe(false);
    expect(composition.view.freshness).toBe("STALE");
  });

  it("treats degraded connectivity as actionable with stale labeling policy intact", () => {
    const composition = DesktopViewState.compose(
      { value: 1 },
      "corr-4",
      "DEGRADED",
    );
    expect(composition.actionable).toBe(true);
    expect(composition.view.freshness).toBe("FRESH");
  });

  it("rejects revalidation to an older revision", () => {
    const current = ViewState.success({ value: 2 }, "corr-5", { revision: 9 });
    const older = ViewState.success({ value: 1 }, "corr-6", { revision: 3 });
    expect(() => DesktopViewState.revalidate(current, older)).toThrowError(
      Spec006Error,
    );
  });

  it("accepts revalidation to a newer revision", () => {
    const current = ViewState.success({ value: 1 }, "corr-7", { revision: 3 });
    const newer = ViewState.success({ value: 2 }, "corr-8", { revision: 4 });
    expect(DesktopViewState.revalidate(current, newer).payload).toEqual({
      value: 2,
    });
  });
});
