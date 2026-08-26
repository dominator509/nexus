/**
 * EP-043 M2 PRODUCTION_READINESS.md renderer (SPEC-008).
 *
 * Renders a deterministic, evidence-backed production readiness report
 * from the pure evaluation result. The report walks every acceptance
 * obligation with exact blocking reasons and never claims readiness
 * that was not proven.
 */

import type { ReadinessEvaluation } from "./readiness.ts";

/** Render the production readiness report as markdown. */
export function renderProductionReadinessReport(
  evaluation: ReadinessEvaluation,
  meta: { node: string; runId: string; gitCommit: string; generatedAt: string },
): string {
  const lines: string[] = [];
  lines.push("# PRODUCTION READINESS");
  lines.push("");
  lines.push(`Node: ${meta.node}`);
  lines.push(`Run: ${meta.runId}`);
  lines.push(`Git commit: ${meta.gitCommit}`);
  lines.push(`Generated: ${meta.generatedAt}`);
  lines.push("");
  lines.push(`## Decision: ${evaluation.decision}`);
  lines.push("");
  lines.push(`Ship gate verdict: ${evaluation.shipGateVerdict}`);
  lines.push("");
  if (evaluation.allMet) {
    lines.push("All acceptance obligations are met. Production readiness is");
    lines.push(
      "DECLARED ONLY FOR the exact exercised surfaces recorded in the",
    );
    lines.push("evidence index. Deployment remains a manual command.");
  } else {
    lines.push("Production readiness is NOT declared. The following blocking");
    lines.push("reasons must be resolved before a ship decision:");
    lines.push("");
    for (const reason of evaluation.blockingReasons) {
      lines.push(`- ${reason}`);
    }
  }
  lines.push("");
  lines.push("## Acceptance Obligations");
  lines.push("");
  for (const obligation of evaluation.obligations) {
    lines.push(`### ${obligation.obligation}`);
    lines.push("");
    lines.push(`Status: ${obligation.met ? "MET" : "NOT MET"}`);
    if (obligation.reasons.length > 0) {
      lines.push("");
      for (const reason of obligation.reasons) {
        lines.push(`- ${reason}`);
      }
    }
    lines.push("");
  }
  lines.push("## Evidence");
  lines.push("");
  lines.push("Evidence is machine-readable and bound to the exact run in");
  lines.push("`.agent/state/evidence/`. Redaction is mandatory; secret-shaped");
  lines.push("content is never written into this report.");
  lines.push("");
  lines.push("## Certification Boundary");
  lines.push("");
  lines.push("This report certifies behavior for the exact exercised local");
  lines.push("surfaces recorded in the evidence index. It does NOT assert:");
  lines.push("- production host upgrades");
  lines.push("- real release signature verification (no key store/verifier)");
  lines.push("- production canary rollout");
  lines.push("- production backup/restore/rollback");
  lines.push("- production deployment");
  lines.push("- AWS/R2/B2 transport");
  lines.push("");
  lines.push("Production deployment is not authorized from the coding graph.");
  lines.push("The exact manual deploy command is recorded in the handoff.");
  return lines.join("\n");
}

/** Redact secret-shaped content from the rendered report. */
export function redactReport(report: string): string {
  return report
    .replace(/sk-[A-Za-z0-9._-]{6,}/g, "[REDACTED]")
    .replace(/AKIA[0-9A-Z._-]{6,}/g, "[REDACTED]")
    .replace(/ghp_[A-Za-z0-9._-]{6,}/g, "[REDACTED]")
    .replace(/Bearer\s+[A-Za-z0-9._-]{16,}/gi, "Bearer [REDACTED]");
}
