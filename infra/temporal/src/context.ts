/**
 * Engine bridge: deterministic workflow environment over the Temporal
 * SDK (ADR-010; SPEC-023 behavior 6).
 *
 * This module is the ONE place workflow code obtains engine time and
 * timers. Temporal's workflow isolate patches the host clock so Date.now()
 * here is deterministic across replay; workflow bodies never call the
 * clock or random APIs directly (enforced by the determinism audit over
 * src/workflows and src/state). This bridge is engine code, not workflow
 * code.
 */

import { sleep as temporalSleep, workflowInfo } from "@temporalio/workflow";

export interface WorkflowEnv {
  readonly workflowId: string;
  readonly runId: string;
  readonly workflowType: string;
  readonly taskQueue: string;
  readonly namespace: string;
  /** Deterministic engine time (isolate-patched clock). */
  now(): Date;
  /** ISO-8601 UTC snapshot of the deterministic engine clock. */
  nowIso(): string;
  sleep(ms: number): Promise<void>;
}

export function createWorkflowEnv(): WorkflowEnv {
  const info = workflowInfo();
  return {
    workflowId: info.workflowId,
    runId: info.runId,
    workflowType: info.workflowType,
    taskQueue: info.taskQueue,
    namespace: info.namespace,
    now: () => new Date(Date.now()),
    nowIso: () => new Date(Date.now()).toISOString(),
    sleep: (ms) => temporalSleep(ms),
  };
}
