/**
 * Determinism audit for workflow source (SPEC-023 behavior 6; EP-006
 * hard invariant: no host-clock, random-source, network, or database
 * calls inside workflow code).
 *
 * The rules live in determinism-rules.json (data, not code) so the audit
 * does not flag its own patterns. findDeterminismViolations is a pure
 * function: source text in, violations out. File walking lives in the
 * test zone (src/__tests__/helpers) and in CI scripts, never in workflow
 * code.
 */

import rulesJson from "./determinism-rules.json" with { type: "json" };

export interface DeterminismRule {
  readonly pattern: string;
  readonly reason: string;
}

export const DETERMINISM_RULES: readonly DeterminismRule[] =
  rulesJson.rules as readonly DeterminismRule[];

export interface DeterminismViolation {
  readonly line: number;
  readonly rule: string;
  readonly reason: string;
  readonly match: string;
}

/** Scan source text for forbidden non-deterministic calls. Pure function. */
export function findDeterminismViolations(
  source: string,
  rules: readonly DeterminismRule[] = DETERMINISM_RULES,
): DeterminismViolation[] {
  const violations: DeterminismViolation[] = [];
  const lines = source.split(/\r?\n/);
  for (const rule of rules) {
    const regex = new RegExp(rule.pattern, "g");
    for (let i = 0; i < lines.length; i += 1) {
      const line = lines[i];
      if (line === undefined) {
        continue;
      }
      regex.lastIndex = 0;
      const match = regex.exec(line);
      if (match !== null) {
        violations.push({
          line: i + 1,
          rule: rule.pattern,
          reason: rule.reason,
          match: match[0],
        });
      }
    }
  }
  return violations;
}

export function formatViolations(
  violations: readonly DeterminismViolation[],
): string {
  if (violations.length === 0) {
    return "no determinism violations";
  }
  return violations
    .map((v) => `line ${v.line}: ${JSON.stringify(v.match)} (${v.reason})`)
    .join("\n");
}
