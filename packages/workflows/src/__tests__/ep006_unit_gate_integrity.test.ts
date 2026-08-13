import { describe, expect, it } from "vitest";

import { readFileSync } from "node:fs";
import path from "node:path";

import { repoRoot } from "./helpers/fixtures.js";

/**
 * Gate-integrity regression guard (EP-001 gate-masking class).
 *
 * The M1 gate must execute the real ep006_unit test suite before printing
 * the milestone sentinel. Two failure modes are forbidden:
 * 1. An artifact-only gate that prints "ok" without running any test.
 * 2. A test filter that matches zero tests but exits 0 (vitest exits 0
 *    on a zero-match name filter), so the gate carries a vacuity guard
 *    that requires at least one passed test in the summary.
 */

const EP006_SH = path.join(repoRoot, "scripts/nodes/EP-006.sh");
const M1_FENCE = path.join(repoRoot, ".agent/milestone-files/EP-006-M1.txt");

describe("ep006_unit_gate_integrity", () => {
  it("ep006_unit_gate_m1_runs_test_suite", () => {
    const script = readFileSync(EP006_SH, "utf8");
    const m1Line = script
      .split("\n")
      .find((line) => line.trim().startsWith("M1)"));
    expect(m1Line).toBeDefined();
    expect(m1Line).toContain("node-artifact-check.py EP-006 M1");
    expect(m1Line).toContain("run_ep006_tests ep006_unit");
    // rc capture: a failing test run must suppress the sentinel.
    expect(m1Line).toContain("rc=$?");
  });

  it("ep006_unit_gate_helper_runs_vitest_with_filter", () => {
    const script = readFileSync(EP006_SH, "utf8");
    expect(script).toContain("run_ep006_tests()");
    expect(script).toContain('vitest run -t "$filter"');
  });

  it("ep006_unit_gate_helper_has_vacuity_guard", () => {
    const script = readFileSync(EP006_SH, "utf8");
    // vitest exits 0 when the name filter matches zero tests; the gate
    // must fail closed unless the summary shows at least one passed
    // test. Pattern covers BOTH summary shapes: "94 passed (94)" and
    // the mixed-shape "15 passed | 94 skipped (109)" (vitest switches
    // to the pipe form when any tests in the run are skipped - the M4
    // failure filter matches some files and skips the rest).
    expect(script).toContain("grep -qE 'Tests[[:space:]]+[1-9][0-9]* passed'");
    expect(script).toContain("vacuity guard");
  });

  it("ep006_unit_gate_sentinel_only_after_tests", () => {
    const script = readFileSync(EP006_SH, "utf8");
    const m1Line = script
      .split("\n")
      .find((line) => line.trim().startsWith("M1)"));
    const sentinelLine = script
      .split("\n")
      .find((line) => line.includes('echo "EP-006 $mode: ok"'));
    expect(m1Line !== undefined && sentinelLine !== undefined).toBe(true);
    // The M1 case must not be the sentinel-only artifact check pattern
    // (i.e. it must chain the test invocation).
    expect(m1Line?.includes("&&")).toBe(true);
  });

  it("ep006_unit_gate_milestone_fence_covers_workflows", () => {
    const fence = readFileSync(M1_FENCE, "utf8");
    expect(fence).toContain("packages/workflows/");
  });

  it("ep006_unit_gate_milestone_fence_covers_vocabulary_adr", () => {
    const fence = readFileSync(M1_FENCE, "utf8");
    expect(fence).toContain("references/ADR-010");
    expect(fence).toContain("docs/vocabulary/README.md");
  });
});
