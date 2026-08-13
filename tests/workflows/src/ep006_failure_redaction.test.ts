/**
 * EP-006 M4: observability - redacted logs (execplan M4 content 4;
 * SECURITY.md: no secret in logs, artifacts, or support bundles).
 *
 * The stack bootstrap redacts the per-stack credential from every
 * diagnostic (server exit evidence, health-poll failure logs). This pure
 * test proves the redaction is total: the secret never appears in
 * redacted output, including when it is embedded mid-token or repeated.
 * No stack is started; the helper under test is pure.
 */

import { describe, expect, it } from "vitest";

import { redact } from "./helpers/stack.js";

describe("ep006_failure_redaction", () => {
  it("ep006_failure_redaction_masks_secret_totally", () => {
    const secret = "deadbeefdeadbeefdeadbeefdeadbeef";
    const text = `password=${secret} user=nexus token=${secret}again`;
    const out = redact(text, secret);
    expect(out).not.toContain(secret);
    expect(out).toContain("<redacted>");
  });

  it("ep006_failure_redaction_masks_repeated_and_adjacent", () => {
    const secret = "a1b2c3d4e5f60718293a4b5c6d7e8f90";
    const out = redact(`${secret}${secret} between ${secret}`, secret);
    expect(out).not.toContain(secret);
    expect(out.match(/<redacted>/g)).toHaveLength(3);
  });

  it("ep006_failure_redaction_no_secret_returns_unchanged", () => {
    const text = "no secrets here";
    expect(redact(text, "not-present")).toBe(text);
  });

  it("ep006_failure_redaction_empty_secret_is_noop", () => {
    const text = "anything";
    expect(redact(text, "")).toBe(text);
  });
});
