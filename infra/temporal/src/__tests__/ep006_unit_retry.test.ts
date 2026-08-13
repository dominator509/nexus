import { describe, expect, it } from "vitest";

import { DEFAULT_RETRY_POLICY, WorkflowContractError } from "@nexus/workflows";
import type { RetryPolicy as NexusRetryPolicy } from "@nexus/workflows";

import { toTemporalRetry } from "../retry.js";

describe("ep006_unit_retry", () => {
  it("ep006_unit_retry_maps_bounded_policy", () => {
    const mapped = toTemporalRetry(DEFAULT_RETRY_POLICY);
    expect(mapped.backoffCoefficient).toBe(
      DEFAULT_RETRY_POLICY.backoffCoefficient,
    );
    expect(mapped.initialInterval).toBe(DEFAULT_RETRY_POLICY.initialIntervalMs);
    expect(mapped.maximumAttempts).toBe(DEFAULT_RETRY_POLICY.maxAttempts);
    expect(mapped.maximumInterval).toBe(DEFAULT_RETRY_POLICY.maxIntervalMs);
  });

  it("ep006_unit_retry_never_infinity", () => {
    const mapped = toTemporalRetry(DEFAULT_RETRY_POLICY);
    expect(mapped.maximumAttempts).toBeLessThan(Number.POSITIVE_INFINITY);
    expect(Number.isFinite(mapped.maximumAttempts)).toBe(true);
  });

  it("ep006_unit_retry_empty_classes_single_attempt", () => {
    const policy: NexusRetryPolicy = {
      ...DEFAULT_RETRY_POLICY,
      retryableErrorClasses: [],
    };
    expect(toTemporalRetry(policy).maximumAttempts).toBe(1);
  });

  it("ep006_unit_retry_rejects_invalid_policy", () => {
    expect(() =>
      toTemporalRetry({ ...DEFAULT_RETRY_POLICY, maxAttempts: 0 }),
    ).toThrow(WorkflowContractError);
  });

  it("ep006_unit_retry_rejects_permanent_retryable", () => {
    expect(() =>
      toTemporalRetry({
        ...DEFAULT_RETRY_POLICY,
        retryableErrorClasses: ["PERMANENT"],
      }),
    ).toThrow(/PERMANENT/);
  });
});
