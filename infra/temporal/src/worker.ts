/**
 * Temporal worker factory (EP-006 fallback doctrine: one namespace, one
 * worker process, task queues separated by capability).
 *
 * EP-006 registers the approval-owned activities; later nodes extend the
 * registry with their provider effects through the `extraActivities`
 * option. An unregistered activity invoked by a workflow fails closed at
 * runtime (typed ActivityNotFound), never silently.
 */

import path from "node:path";
import { fileURLToPath } from "node:url";

import { NativeConnection, Runtime, Worker } from "@temporalio/worker";

import { applyCompensation, verifyApproval } from "./activities.js";
import type { NexusActivityRegistry } from "./activity-types.js";
import { NAMESPACE } from "./config.js";

export interface TemporalWorkerOptions {
  readonly address?: string;
  readonly namespace?: string;
  readonly taskQueues: readonly string[];
  readonly extraActivities?: Partial<NexusActivityRegistry>;
  readonly workflowsPath?: string;
}

const CORE_ACTIVITIES: Partial<NexusActivityRegistry> = {
  verifyApproval,
  applyCompensation,
};

/** Default workflow bundle path (this package's src/workflows). */
export function defaultWorkflowsPath(): string {
  return path.join(path.dirname(fileURLToPath(import.meta.url)), "workflows");
}

export interface StartedWorker {
  readonly worker: Worker;
  readonly connection: NativeConnection;
  shutdown(): Promise<void>;
}

/**
 * Create a worker serving the given task queues with the nexus workflows
 * and the merged activity registry. `runtime` and `connection` are owned
 * by the caller (composition root); this factory never creates its own
 * runtime (EP-005 owner doctrine).
 */
export async function createTemporalWorker(
  options: TemporalWorkerOptions,
  runtime: Runtime,
  connection: NativeConnection,
): Promise<StartedWorker> {
  void runtime;
  const activities: Partial<NexusActivityRegistry> = {
    ...CORE_ACTIVITIES,
    ...options.extraActivities,
  };
  const worker = await Worker.create({
    namespace: options.namespace ?? NAMESPACE,
    taskQueue: options.taskQueues[0] as string,
    workflowsPath: options.workflowsPath ?? defaultWorkflowsPath(),
    activities,
    connection,
  });
  return {
    worker,
    connection,
    shutdown: async () => {
      await worker.shutdown();
      await connection.close();
    },
  };
}

export { Runtime, NativeConnection };
