import { describe, expect, it } from "vitest";

import { NexusWorkflowError } from "@nexus/workflows";

import { toApplicationFailure } from "../failure.js";
import { NexusFailureInterceptor } from "../interceptors.js";

describe("ep006_unit_failure", () => {
  it("ep006_unit_failure_permanent_code_is_non_retryable", () => {
    const err = new NexusWorkflowError("VALIDATION", "bad input");
    const af = toApplicationFailure(err);
    expect(af.type).toBe("VALIDATION");
    expect(af.nonRetryable).toBe(true);
  });

  it("ep006_unit_failure_transient_code_is_retryable", () => {
    const err = new NexusWorkflowError("UNAVAILABLE", "provider down");
    const af = toApplicationFailure(err);
    expect(af.type).toBe("UNAVAILABLE");
    expect(af.nonRetryable).toBe(false);
  });

  it("ep006_unit_failure_interceptor_classifies_nexus_errors", async () => {
    const interceptor = new NexusFailureInterceptor();
    // Test double for the SDK call chain: a next() that throws the typed
    // failure exactly as an activity would.
    const next = async (): Promise<unknown> => {
      throw new NexusWorkflowError("POLICY", "denied");
    };
    await expect(
      interceptor.execute({ args: [], headers: {} }, next),
    ).rejects.toMatchObject({ type: "POLICY", nonRetryable: true });
  });

  it("ep006_unit_failure_interceptor_passes_through_other_errors", async () => {
    const interceptor = new NexusFailureInterceptor();
    const boom = new Error("boom");
    const next = async (): Promise<unknown> => {
      throw boom;
    };
    await expect(
      interceptor.execute({ args: [], headers: {} }, next),
    ).rejects.toBe(boom);
  });

  it("ep006_unit_failure_interceptor_returns_result", async () => {
    const interceptor = new NexusFailureInterceptor();
    const next = async (): Promise<unknown> => 42;
    await expect(
      interceptor.execute({ args: [], headers: {} }, next),
    ).resolves.toBe(42);
  });
});
