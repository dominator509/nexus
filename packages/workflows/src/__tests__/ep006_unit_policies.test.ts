import { describe, expect, it } from "vitest";

import { WorkflowContractError } from "../errors.js";
import {
  DEFAULT_RETRY_POLICY,
  validateRetryPolicy,
  validateTimeoutPolicy,
  validateWorkflowPolicy,
} from "../policies.js";
import type { WorkflowPolicy } from "../policies.js";
import { cancelAction } from "../vocabulary.js";

const VALID_POLICY: WorkflowPolicy = {
  timeouts: {
    executionTimeoutMs: 30 * 24 * 60 * 60 * 1000, // 30 days
    runTimeoutMs: 7 * 24 * 60 * 60 * 1000,
    taskTimeoutMs: 10 * 60 * 1000,
    approvalTimeoutMs: 5 * 24 * 60 * 60 * 1000,
  },
  cancelAction: "COMPENSATE",
  defaultActivityRetry: DEFAULT_RETRY_POLICY,
};

describe("ep006_unit_policies", () => {
  it("ep006_unit_policies_valid_policy_accepted", () => {
    expect(() => validateWorkflowPolicy(VALID_POLICY)).not.toThrow();
  });

  it("ep006_unit_policies_accepts_cancel_semantics", () => {
    expect(() =>
      validateWorkflowPolicy({ ...VALID_POLICY, cancelAction: "CANCEL" }),
    ).not.toThrow();
    expect(cancelAction.parse("COMPENSATE")).toBe("COMPENSATE");
    expect(cancelAction.parse("CANCEL")).toBe("CANCEL");
  });

  it("ep006_unit_policies_rejects_zero_attempts", () => {
    expect(() =>
      validateRetryPolicy({ ...DEFAULT_RETRY_POLICY, maxAttempts: 0 }),
    ).toThrow(WorkflowContractError);
  });

  it("ep006_unit_policies_rejects_unbounded_retries", () => {
    // SPEC-006 behavior 7: retries are bounded, never forever.
    expect(() =>
      validateRetryPolicy({
        ...DEFAULT_RETRY_POLICY,
        maxAttempts: Number.POSITIVE_INFINITY,
      }),
    ).toThrow(WorkflowContractError);
  });

  it("ep006_unit_policies_rejects_permanent_retryable", () => {
    expect(() =>
      validateRetryPolicy({
        ...DEFAULT_RETRY_POLICY,
        retryableErrorClasses: ["PERMANENT"],
      }),
    ).toThrow(/PERMANENT/);
  });

  it("ep006_unit_policies_rejects_zero_initial_interval", () => {
    expect(() =>
      validateRetryPolicy({ ...DEFAULT_RETRY_POLICY, initialIntervalMs: 0 }),
    ).toThrow(WorkflowContractError);
  });

  it("ep006_unit_policies_rejects_max_less_than_initial", () => {
    expect(() =>
      validateRetryPolicy({
        ...DEFAULT_RETRY_POLICY,
        initialIntervalMs: 5000,
        maxIntervalMs: 1000,
      }),
    ).toThrow(WorkflowContractError);
  });

  it("ep006_unit_policies_rejects_run_gt_execution", () => {
    expect(() =>
      validateTimeoutPolicy({
        executionTimeoutMs: 1000,
        runTimeoutMs: 2000,
        taskTimeoutMs: 500,
      }),
    ).toThrow(WorkflowContractError);
  });

  it("ep006_unit_policies_rejects_task_gt_run", () => {
    expect(() =>
      validateTimeoutPolicy({
        executionTimeoutMs: 1000,
        runTimeoutMs: 1000,
        taskTimeoutMs: 2000,
      }),
    ).toThrow(WorkflowContractError);
  });

  it("ep006_unit_policies_rejects_nonpositive_execution_timeout", () => {
    expect(() =>
      validateTimeoutPolicy({
        executionTimeoutMs: 0,
        runTimeoutMs: 0,
        taskTimeoutMs: 0,
      }),
    ).toThrow(WorkflowContractError);
  });

  it("ep006_unit_policies_rejects_bad_cancel_action", () => {
    expect(() =>
      validateWorkflowPolicy({
        ...VALID_POLICY,
        cancelAction: "IGNORE" as never,
      }),
    ).toThrow(WorkflowContractError);
  });

  it("ep006_unit_policies_rejects_bad_approval_timeout", () => {
    expect(() =>
      validateTimeoutPolicy({
        ...VALID_POLICY.timeouts,
        approvalTimeoutMs: -1,
      }),
    ).toThrow(WorkflowContractError);
  });
});
