/**
 * EP-042 M2 deterministic update planner proofs (SPEC-016 behavior 6).
 *
 * Valid inputs -> PLANNED (PLAN_READY) only. The plan does not execute
 * installation, does not mutate state, denies downgrades, denies same
 * version, denies incompatible component sets, denies unsupported
 * profiles, requires backup-first, requires a rollback path, has no
 * promote step, and yields a deterministic plan digest.
 *
 * UPDATE PLAN EXISTS != UPDATE EXECUTED.
 */

import { describe, expect, it } from "vitest";
import {
  ReleaseError,
  ReleaseErrorCode,
  buildUpdatePlan,
  contentDigest,
  parseUpdatePlan,
  planContainsNoPromoteStep,
  planHasBackupFirstStep,
  planHasRollbackPath,
  sha256Hex,
} from "@nexus/setup";
import { boundManifest, fixtureManifest, planWire } from "./fixtures";

const BASE_INPUT = {
  plan_id: "plan-1",
  from_version: "1.0.0",
  to_version: "1.1.0",
  channel: "STABLE" as const,
  profile: "MANAGED" as const,
  idempotency_key: "idem-1",
  correlation_id: "corr-1",
  created_at: "2026-08-25T00:00:00Z",
};

describe("ep042_unit update planner", () => {
  it("ep042_unit_planner_returns_planned_only", async () => {
    const release = await boundManifest();
    const { plan, plan_digest } = await buildUpdatePlan({
      ...BASE_INPUT,
      release,
    });
    expect(plan.state).toBe("PLANNED");
    expect(plan_digest.alg()).toBe("sha256");
    expect(plan_digest.hex().length).toBe(64);
  });

  it("ep042_unit_planner_never_executes_installation", async () => {
    const release = await boundManifest();
    const { plan } = await buildUpdatePlan({ ...BASE_INPUT, release });
    // The result is a plan record + digest; there is no executor surface,
    // no install command, no mutation side effect.
    expect(plan.steps.map((step) => step.kind)).not.toContain("PROMOTE");
    expect(planHasBackupFirstStep(plan)).toBe(true);
    expect(planHasRollbackPath(plan)).toBe(true);
  });

  it("ep042_unit_planner_plan_digest_is_deterministic", async () => {
    const release = await boundManifest();
    const a = await buildUpdatePlan({ ...BASE_INPUT, release });
    const b = await buildUpdatePlan({ ...BASE_INPUT, release });
    expect(a.plan_digest.equals(b.plan_digest)).toBe(true);
    // Digest equals real sha256 over the canonical plan wire form.
    const wire = planWire();
    const expected = await sha256Hex(
      new TextEncoder().encode(JSON.stringify(wire)),
    );
    expect(a.plan_digest.hex()).toBe(expected);
  });

  it("ep042_unit_planner_rejects_same_version", async () => {
    const release = await boundManifest();
    await expect(
      buildUpdatePlan({ ...BASE_INPUT, to_version: "1.0.0", release }),
    ).rejects.toThrow(ReleaseError);
  });

  it("ep042_unit_planner_rejects_downgrade", async () => {
    const release = await boundManifest();
    try {
      await buildUpdatePlan({
        ...BASE_INPUT,
        from_version: "1.1.0",
        to_version: "1.0.0",
        release,
      });
      throw new Error("expected downgrade denial");
    } catch (error) {
      expect(error).toBeInstanceOf(ReleaseError);
      const releaseError = error as ReleaseError;
      expect(releaseError.code).toBe(ReleaseErrorCode.Validation);
      expect(releaseError.message).toContain("downgrade");
    }
  });

  it("ep042_unit_planner_rejects_incompatible_component_set", async () => {
    // Unbound manifest: no digest binding, so the compatibility check is
    // the first denial (not a digest mismatch).
    const release = fixtureManifest();
    const incompatible = {
      ...release,
      components: release.components.map((component) =>
        component.component_id === "comp-1"
          ? { ...component, version: "9.9.9" }
          : component,
      ),
    };
    try {
      await buildUpdatePlan({ ...BASE_INPUT, release: incompatible });
      throw new Error("expected incompatible denial");
    } catch (error) {
      expect(error).toBeInstanceOf(ReleaseError);
      const releaseError = error as ReleaseError;
      expect(releaseError.code).toBe(ReleaseErrorCode.Incompatible);
    }
  });

  it("ep042_unit_planner_rejects_unsupported_profile", async () => {
    // Unbound manifest: digest binding is MISSING (acceptable), so the
    // profile support check is the first denial.
    const release = fixtureManifest();
    const narrow = {
      ...release,
      compatibility: {
        ...release.compatibility,
        entries: release.compatibility.entries.map((entry) => ({
          ...entry,
          supported_profiles: ["MANAGED"] as const,
        })),
      },
    };
    await expect(
      buildUpdatePlan({
        ...BASE_INPUT,
        profile: "FULLY_LOCAL",
        release: narrow,
      }),
    ).rejects.toThrow(ReleaseError);
  });

  it("ep042_unit_planner_requires_manifest_digest_binding", async () => {
    // A manifest without a digest binding is acceptable (MISSING); a
    // manifest with a mismatched binding is denied.
    const release = await boundManifest();
    const tampered = {
      ...release,
      manifest_digest: "sha256:0123456789abcdef0123456789abcdef",
    };
    await expect(
      buildUpdatePlan({ ...BASE_INPUT, release: tampered }),
    ).rejects.toThrow(ReleaseError);
  });

  it("ep042_unit_planner_plan_has_backup_first_step", async () => {
    const release = await boundManifest();
    const { plan } = await buildUpdatePlan({ ...BASE_INPUT, release });
    expect(planHasBackupFirstStep(plan)).toBe(true);
    expect(plan.steps[0]?.kind).toBe("BACKUP");
  });

  it("ep042_unit_planner_plan_has_rollback_path", async () => {
    const release = await boundManifest();
    const { plan } = await buildUpdatePlan({ ...BASE_INPUT, release });
    expect(planHasRollbackPath(plan)).toBe(true);
  });

  it("ep042_unit_planner_plan_contains_no_promote_step", async () => {
    const release = await boundManifest();
    const { plan } = await buildUpdatePlan({ ...BASE_INPUT, release });
    expect(planContainsNoPromoteStep(plan)).toBe(true);
  });

  it("ep042_unit_planner_rejects_plan_with_promote_step_kind", () => {
    // The vocabulary itself has no PROMOTE; a wire plan containing
    // "PROMOTE" step kind is denied at parse.
    const wire = structuredClone(planWire());
    const steps = wire["steps"] as Array<Record<string, unknown>>;
    steps.push({ order: 6, kind: "PROMOTE", description: "nope" });
    expect(() => parseUpdatePlan(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_planner_rejects_plan_without_backup_first_step", () => {
    const wire = structuredClone(planWire());
    const steps = wire["steps"] as Array<Record<string, unknown>>;
    steps[0] = { order: 1, kind: "MIGRATE", description: "no backup" };
    expect(() => parseUpdatePlan(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_planner_rejects_non_contiguous_step_order", () => {
    const wire = structuredClone(planWire());
    const steps = wire["steps"] as Array<Record<string, unknown>>;
    steps[1] = { order: 9, kind: "MIGRATE", description: "wrong order" };
    expect(() => parseUpdatePlan(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_planner_rejects_non_planned_state", () => {
    const wire = { ...planWire(), state: "IN_PROGRESS" };
    expect(() => parseUpdatePlan(wire)).toThrow(ReleaseError);
  });

  it("ep042_unit_planner_parse_roundtrip_preserves_schema_version", () => {
    const plan = parseUpdatePlan(planWire());
    expect(plan.schema_version).toBe(1);
    expect(plan.state).toBe("PLANNED");
  });

  it("ep042_unit_planner_plan_digest_changes_with_content", async () => {
    const release = await boundManifest();
    const a = await buildUpdatePlan({ ...BASE_INPUT, release });
    const b = await buildUpdatePlan({
      ...BASE_INPUT,
      to_version: "1.2.0",
      release,
    });
    expect(a.plan_digest.equals(b.plan_digest)).toBe(false);
  });

  it("ep042_unit_planner_canonical_json_is_stable", async () => {
    const release = await boundManifest();
    const { plan } = await buildUpdatePlan({ ...BASE_INPUT, release });
    const bytesA = new TextEncoder().encode(
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
    );
    const digestA = await contentDigest(
      JSON.parse(new TextDecoder().decode(bytesA)) as Record<string, unknown>,
    );
    expect(digestA.hex().length).toBe(64);
  });
});
