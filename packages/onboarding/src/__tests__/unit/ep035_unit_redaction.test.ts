/**
 * EP-035 M3 unit tests: pure integration-layer logic (no containers).
 */

import { describe, expect, it } from "vitest";
import {
  isSecretShaped,
  redactErrorDetail,
  redactSecrets,
  safeSummary,
} from "../../redact.js";
import { hashSecret } from "../../stores/enrollment-token.store.js";

describe("ep035_unit_redaction", () => {
  it("classifies secret-shaped strings", () => {
    expect(
      isSecretShaped("bootstrap_secret_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
    ).toBe(true);
    expect(isSecretShaped("sk_live_abcdefghijklmnopqrstuvwx")).toBe(true);
    expect(isSecretShaped("nexus_abcdefghijklmnopqrstuv")).toBe(true);
    expect(isSecretShaped("hello")).toBe(false);
    expect(isSecretShaped("just a normal sentence")).toBe(false);
  });

  it("redacts secrets but preserves safe text", () => {
    const input =
      "claim ok token=bootstrap_secret_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA done";
    const out = redactSecrets(input);
    expect(out).not.toContain(
      "bootstrap_secret_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    );
    expect(out).toContain("claim ok");
    expect(out).toContain("done");
  });

  it("safeSummary never leaks secret-shaped values", () => {
    const summary = safeSummary({
      correlation_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073",
      secret: "bootstrap_secret_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
      state: "ISSUED",
    });
    expect(summary).not.toContain(
      "bootstrap_secret_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    );
    expect(summary).toContain("ISSUED");
  });

  it("redactErrorDetail strips secret material from failure text", () => {
    const detail = redactErrorDetail(
      "claim failed: bootstrap_secret_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
    );
    expect(detail).not.toContain(
      "bootstrap_secret_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
    );
    expect(detail).toContain("claim failed");
  });

  it("hashSecret is deterministic and never contains the raw value", () => {
    const secret = "bootstrap_secret_CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC";
    const h1 = hashSecret(secret);
    const h2 = hashSecret(secret);
    expect(h1).toBe(h2);
    expect(h1).not.toContain(secret);
    expect(h1).toMatch(/^[0-9a-f]{64}$/);
  });
});
