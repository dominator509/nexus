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

  it("ep006_unit_retry_declares_permanent_types_non_retryable", () => {
    const mapped = toTemporalRetry(DEFAULT_RETRY_POLICY);
    expect(mapped.nonRetryableErrorTypes).toBeDefined();
    // Permanent SPEC-006 codes are declared non-retryable: they must
    // never consume the five attempts (AUD-023).
    for (const code of [
      "VALIDATION",
      "AUTHENTICATION",
      "AUTHORIZATION",
      "POLICY",
      "EXTERNAL_PROVIDER",
      "VERIFICATION",
      "COMPENSATION",
      "INTERNAL_INVARIANT",
    ]) {
      expect(mapped.nonRetryableErrorTypes).toContain(code);
    }
    // Transient codes stay retryable under the default policy.
    for (const code of ["UNAVAILABLE", "TIMEOUT", "RATE_LIMIT", "CONFLICT"]) {
      expect(mapped.nonRetryableErrorTypes).not.toContain(code);
    }
  });

  it("ep006_unit_retry_narrow_policy_excludes_unlisted_classes", () => {
    const policy: NexusRetryPolicy = {
      ...DEFAULT_RETRY_POLICY,
      retryableErrorClasses: ["TIMEOUT"],
    };
    const mapped = toTemporalRetry(policy);
    // UNAVAILABLE is not in the narrow retryable list -> non-retryable.
    expect(mapped.nonRetryableErrorTypes).toContain("UNAVAILABLE");
    expect(mapped.nonRetryableErrorTypes).toContain("VALIDATION");
    // TIMEOUT is the only listed class -> its code stays retryable.
    expect(mapped.nonRetryableErrorTypes).not.toContain("TIMEOUT");
  });

  it("ep006_unit_retry_empty_classes_single_attempt_all_non_retryable", () => {
    const policy: NexusRetryPolicy = {
      ...DEFAULT_RETRY_POLICY,
      retryableErrorClasses: [],
    };
    const mapped = toTemporalRetry(policy);
    expect(mapped.maximumAttempts).toBe(1);
    expect(mapped.nonRetryableErrorTypes).toHaveLength(
      // All SPEC-006 codes are excluded when no class is retryable.
      [
        "VALIDATION",
        "AUTHENTICATION",
        "AUTHORIZATION",
        "POLICY",
        "UNAVAILABLE",
        "TIMEOUT",
        "CONFLICT",
        "RATE_LIMIT",
        "EXTERNAL_PROVIDER",
        "VERIFICATION",
        "COMPENSATION",
        "INTERNAL_INVARIANT",
      ].length,
    );
  });
});
