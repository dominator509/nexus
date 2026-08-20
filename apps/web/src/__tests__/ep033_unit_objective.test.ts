import { describe, expect, it } from "vitest";
import {
  ObjectiveView,
  TaskNode,
  OBJECTIVE_STAGES,
} from "../contracts/objective-view";
import { ErrorCode, Spec006Error } from "../contracts/errors";

describe("ep033_unit_objective_view", () => {
  it("constructs an objective with typed ids and correlation", () => {
    const objective = ObjectiveView.fromWire({
      objective_id: "obj-0001",
      title: "Prepare quarterly review",
      stage: "ACTIVE",
      correlation_id: "corr-0001",
      owner_principal_id: "00000000-0000-4000-8000-000000000002",
    });
    expect(objective.stage).toBe("ACTIVE");
    expect(objective.matchesCorrelation("corr-0001")).toBe(true);
  });

  it("exposes canonical objective stages", () => {
    expect([...OBJECTIVE_STAGES]).toEqual([
      "PROPOSED",
      "ACTIVE",
      "AWAITING_APPROVAL",
      "BLOCKED",
      "DONE",
      "CANCELLED",
    ]);
  });

  it("rejects unsupported objective stages", () => {
    expect(() =>
      ObjectiveView.fromWire({
        objective_id: "obj-0001",
        title: "x",
        stage: "MYSTERY",
        correlation_id: "corr-0001",
        owner_principal_id: "00000000-0000-4000-8000-000000000002",
      }),
    ).toThrowError(Spec006Error);
  });

  it("rejects empty objective ids", () => {
    expect(() =>
      ObjectiveView.fromWire({
        objective_id: "",
        title: "x",
        stage: "ACTIVE",
        correlation_id: "corr-0001",
        owner_principal_id: "00000000-0000-4000-8000-000000000002",
      }),
    ).toThrowError(Spec006Error);
  });

  it("rejects unknown fields", () => {
    expect(() =>
      ObjectiveView.fromWire({
        objective_id: "obj-0001",
        title: "x",
        stage: "ACTIVE",
        correlation_id: "corr-0001",
        owner_principal_id: "00000000-0000-4000-8000-000000000002",
        progress_percent: 42,
      }),
    ).toThrowError(Spec006Error);
  });

  it("provides the continuity seam: same correlation binds the task graph", () => {
    // LF-005 seam: an objective started by voice carries a correlation
    // that the web objective view binds to; the view displays stage, it
    // never advances it.
    const objective = ObjectiveView.fromWire({
      objective_id: "obj-0001",
      title: "Start by voice",
      stage: "ACTIVE",
      correlation_id: "voice-corr-0001",
      owner_principal_id: "00000000-0000-4000-8000-000000000002",
    });
    expect(objective.matchesCorrelation("voice-corr-0001")).toBe(true);
    expect(objective.matchesCorrelation("other")).toBe(false);
  });

  it("constructs task nodes bound to their objective", () => {
    const task = new TaskNode("task-0001", "obj-0001", "Draft outline", false);
    expect(task.objective_id).toBe("obj-0001");
    expect(task.done).toBe(false);
  });

  it("rejects task nodes with empty ids", () => {
    expect(
      () => new TaskNode("", "obj-0001", "Draft outline", false),
    ).toThrowError(Spec006Error);
  });
});
