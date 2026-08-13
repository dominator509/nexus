/**
 * EP-006 M3 worker helper: start the nexus workers over the session's
 * owned Runtime + NativeConnection (EP-005 owner doctrine). Bundles the
 * workflow code once per process so the worker serves the canonical
 * workflow type names (nexus.*.v1).
 *
 * Worker lifecycle is a teardown invariant: every started worker is shut
 * down explicitly and awaited by the caller (try/finally), and any
 * worker run() rejection is surfaced by shutdown() - never swallowed.
 * A worker whose slots stay registered poisons later workers in this
 * process (SDK-level task-queue slot overlap), so leaked workers are a
 * hard failure, not a warning.
 */

import {
  TASK_QUEUES,
  createTemporalWorker,
  defaultWorkflowsPath,
} from "@nexus/temporal";
import type { NexusActivityRegistry } from "@nexus/temporal";
import { bundleWorkflowCode } from "@temporalio/worker";
import { randomUUID } from "node:crypto";

import type { getSession } from "./session.js";

export type TestSession = Awaited<ReturnType<typeof getSession>>;

export interface StartedWorkers {
  /**
   * Explicit, awaited worker teardown. Throws if any worker run()
   * rejected or any worker failed to stop - silent worker death is not
   * acceptable (leaked task-queue slots break every later worker).
   */
  readonly shutdown: () => Promise<void>;
}

let cachedBundle: Awaited<ReturnType<typeof bundleWorkflowCode>> | undefined;
let workerSeq = 0;

export async function workflowBundle(): Promise<
  Awaited<ReturnType<typeof bundleWorkflowCode>>
> {
  if (cachedBundle === undefined) {
    cachedBundle = await bundleWorkflowCode({
      workflowsPath: defaultWorkflowsPath(),
    });
  }
  return cachedBundle;
}

/**
 * Start workers for the given workflow task queues (plus the shared
 * activity queue) over the session connection. Test activities for
 * provider-bound effects (runEffect/verifyEffect) may be registered in
 * the TESTING.md test zone.
 *
 * NOTE (verified 2026-08-13 against SDK 1.17.2 source): a unique buildId
 * is a stable identifier only. With useVersioning: false the SDK derives
 * the deployment slot from deployment_options() (None), so buildId does
 * NOT prevent the "multiple workers with overlapping worker task types"
 * conflict. The real prevention is orderly, awaited shutdown of every
 * previous worker before starting new ones - never a leaked worker.
 */
export async function startWorker(
  session: TestSession,
  taskQueues: readonly string[],
  extraActivities?: Partial<NexusActivityRegistry>,
): Promise<StartedWorkers> {
  const bundle = await workflowBundle();
  workerSeq += 1;
  const buildId = `nexus-test-${process.pid}-${workerSeq}-${randomUUID()}`;
  const started = await createTemporalWorker(
    {
      namespace: session.namespace,
      taskQueues: [...taskQueues, TASK_QUEUES.ACTIVITY],
      workflowBundle: bundle,
      buildId,
      ...(extraActivities === undefined ? {} : { extraActivities }),
    },
    session.runtime,
    session.connection,
  );
  const runs = started.workers.map((worker) => worker.run());
  const runFailures: string[] = [];
  for (const run of runs) {
    run.catch((error) => {
      runFailures.push(String(error));
    });
  }
  return {
    shutdown: async () => {
      await started.shutdown();
      const settled = await Promise.allSettled(runs);
      for (const result of settled) {
        if (result.status === "rejected") {
          runFailures.push(String(result.reason));
        }
      }
      if (runFailures.length > 0) {
        throw new Error(
          `worker shutdown: ${runFailures.length} worker run failure(s):\n${runFailures
            .map((failure) => `- ${failure}`)
            .join("\n")}`,
        );
      }
    },
  };
}
