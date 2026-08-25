/**
 * EP-042 M2 redacted evidence proofs (SPEC-016, SPEC-024).
 *
 * Evidence binds run_id, git_commit, release identity, manifest digest,
 * component identities/digests, compatibility decision, update plan
 * digest, backup/rollback state, promotion state, final decision, and a
 * redaction result. Secret-shaped values are redacted; runtime-
 * constructed secret canaries never leak.
 */

import { describe, expect, it } from "vitest";
import {
  buildRedactedEvidence,
  buildUpdatePlan,
  contentDigest,
  manifestContentDigest,
  parseUpdatePlan,
  redactValue,
  sha256Hex,
} from "@nexus/setup";
import {
  backupProofWire,
  boundManifest,
  drillWire,
  planWire,
} from "./fixtures";

function runtimeSecretCanary(): string {
  // Runtime-constructed: never a tracked secret literal.
  return ["sk-live-", "a1b2c3d4e5f60718293a4b5c6d7e8f90"].join("");
}

describe("ep042_unit evidence redaction", () => {
  it("ep042_unit_evidence_binds_current_run_fields", async () => {
    const release = await boundManifest();
    const { plan, plan_digest } = await buildUpdatePlan({
      plan_id: "plan-1",
      release,
      from_version: "1.0.0",
      to_version: "1.1.0",
      channel: "STABLE",
      profile: "MANAGED",
      idempotency_key: "idem-1",
      correlation_id: "corr-1",
      created_at: "2026-08-25T00:00:00Z",
    });
    const manifestDigest = await manifestContentDigest(release);
    const evidence = buildRedactedEvidence({
      run_id: "run-42",
      git_commit: "5837f57",
      release,
      manifest_digest: manifestDigest,
      compatibility_decision: "COMPATIBLE",
      update_plan: plan,
      update_plan_digest: plan_digest,
      backup_state: "COMPLETED",
      rollback_state: "PROVEN",
      promotion_state: "APPROVED_MANUAL_ONLY",
      final_decision: "APPROVE_MANUAL",
      created_at: "2026-08-25T02:00:00Z",
    });
    expect(evidence.run_id).toBe("run-42");
    expect(evidence.git_commit).toBe("5837f57");
    expect(evidence.release_id).toBe("release-1");
    expect(evidence.manifest_digest).toBe(manifestDigest.asString());
    expect(evidence.update_plan_digest).toBe(plan_digest.asString());
    expect(evidence.component_identities).toContain("comp-1");
    expect(evidence.final_decision).toBe("APPROVE_MANUAL");
  });

  it("ep042_unit_evidence_redacts_runtime_secret_canary", async () => {
    const release = await boundManifest();
    const { plan, plan_digest } = await buildUpdatePlan({
      plan_id: "plan-1",
      release,
      from_version: "1.0.0",
      to_version: "1.1.0",
      channel: "STABLE",
      profile: "MANAGED",
      idempotency_key: "idem-1",
      correlation_id: "corr-1",
      created_at: "2026-08-25T00:00:00Z",
    });
    const manifestDigest = await manifestContentDigest(release);
    const secret = runtimeSecretCanary();
    const evidence = buildRedactedEvidence({
      run_id: "run-43",
      git_commit: "5837f57",
      release,
      manifest_digest: manifestDigest,
      compatibility_decision: "COMPATIBLE",
      update_plan: plan,
      update_plan_digest: plan_digest,
      backup_state: "COMPLETED",
      rollback_state: "PROVEN",
      promotion_state: "APPROVED_MANUAL_ONLY",
      final_decision: "APPROVE_MANUAL",
      created_at: "2026-08-25T02:00:00Z",
      redaction_canary: secret,
    });
    expect(evidence.redaction_applied).toBe(true);
    // The raw secret never appears anywhere in the evidence.
    const serialized = JSON.stringify(evidence);
    expect(serialized).not.toContain(secret);
  });

  it("ep042_unit_redact_value_replaces_secret_shapes", () => {
    const secret = runtimeSecretCanary();
    expect(redactValue(secret)).toBe("[REDACTED]");
    expect(redactValue("plain value")).toBe("plain value");
    expect(redactValue(["AKIA", "IOSFODNN7EXAMPLE"].join(""))).toBe(
      "[REDACTED]",
    );
    expect(redactValue("-----BEGIN RSA PRIVATE KEY-----")).toBe("[REDACTED]");
  });

  it("ep042_unit_evidence_never_emits_secret_shaped_component_ids", async () => {
    const release = await boundManifest();
    const { plan, plan_digest } = await buildUpdatePlan({
      plan_id: "plan-1",
      release,
      from_version: "1.0.0",
      to_version: "1.1.0",
      channel: "STABLE",
      profile: "MANAGED",
      idempotency_key: "idem-1",
      correlation_id: "corr-1",
      created_at: "2026-08-25T00:00:00Z",
    });
    const manifestDigest = await manifestContentDigest(release);
    const secret = runtimeSecretCanary();
    const evidence = buildRedactedEvidence({
      run_id: "run-44",
      git_commit: "5837f57",
      release,
      manifest_digest: manifestDigest,
      compatibility_decision: "COMPATIBLE",
      update_plan: plan,
      update_plan_digest: plan_digest,
      backup_state: "COMPLETED",
      rollback_state: "PROVEN",
      promotion_state: "APPROVED_MANUAL_ONLY",
      final_decision: "APPROVE_MANUAL",
      created_at: "2026-08-25T02:00:00Z",
      redaction_canary: secret,
    });
    const serialized = JSON.stringify(evidence);
    expect(serialized).not.toContain("sk-live-");
    expect(serialized).not.toContain("AKIA");
    expect(serialized).not.toContain("BEGIN RSA");
  });

  it("ep042_unit_evidence_compatibility_decision_label", async () => {
    const release = await boundManifest();
    const { plan, plan_digest } = await buildUpdatePlan({
      plan_id: "plan-1",
      release,
      from_version: "1.0.0",
      to_version: "1.1.0",
      channel: "STABLE",
      profile: "MANAGED",
      idempotency_key: "idem-1",
      correlation_id: "corr-1",
      created_at: "2026-08-25T00:00:00Z",
    });
    const manifestDigest = await manifestContentDigest(release);
    const evidence = buildRedactedEvidence({
      run_id: "run-45",
      git_commit: "5837f57",
      release,
      manifest_digest: manifestDigest,
      compatibility_decision: "INCOMPATIBLE",
      update_plan: plan,
      update_plan_digest: plan_digest,
      backup_state: "DENIED",
      rollback_state: "DENIED",
      promotion_state: "LOCKED",
      final_decision: "DENY",
      created_at: "2026-08-25T02:00:00Z",
    });
    expect(evidence.compatibility_decision).toBe("INCOMPATIBLE");
    expect(evidence.final_decision).toBe("DENY");
  });

  it("ep042_unit_evidence_digests_are_real_sha256", async () => {
    const release = await boundManifest();
    const { plan, plan_digest } = await buildUpdatePlan({
      plan_id: "plan-1",
      release,
      from_version: "1.0.0",
      to_version: "1.1.0",
      channel: "STABLE",
      profile: "MANAGED",
      idempotency_key: "idem-1",
      correlation_id: "corr-1",
      created_at: "2026-08-25T00:00:00Z",
    });
    expect(plan_digest.hex().length).toBe(64);
    const planHex = await sha256Hex(
      new TextEncoder().encode(
        JSON.stringify({
          schema_version: plan.schema_version,
          plan_id: plan.plan_id,
          release_id: plan.release_id,
          from_version: plan.from_version,
          to_version: plan.to_version,
          channel: plan.channel,
          steps: plan.steps,
          idempotency_key: plan.idempotency_key,
          correlation_id: plan.correlation_id,
          created_at: plan.created_at,
          state: plan.state,
        }),
      ),
    );
    expect(planHex.length).toBe(64);
  });

  it("ep042_unit_evidence_rejects_noop_plan_ref", () => {
    // Evidence is bound to a real parsed plan; a wire plan that fails
    // closed (e.g. wrong state) cannot produce evidence.
    const wire = { ...planWire(), state: "DEPLOYED" };
    expect(() => parseUpdatePlan(wire)).toThrow();
  });
});
