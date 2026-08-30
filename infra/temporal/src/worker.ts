/**
 * Temporal worker factory (EP-006 fallback doctrine: one namespace, one
 * worker process, task queues separated by capability).
 *
 * A Temporal Worker polls exactly ONE task queue. The nexus workflows
 * schedule their activities on TASK_QUEUES.ACTIVITY, so this factory
 * creates one Worker per requested workflow queue plus one activity
 * Worker for the shared activity queue - all sharing one connection
 * (EP-005 owner doctrine: the caller owns Runtime + NativeConnection).
 *
 * EP-006 registers the approval-owned activities; later nodes extend the
 * registry with their provider effects through the `extraActivities`
 * option. An unregistered activity invoked by a workflow fails closed at
 * runtime (typed ActivityNotFound), never silently.
 */

import path from "node:path";
import { fileURLToPath } from "node:url";

import { NativeConnection, Runtime, Worker } from "@temporalio/worker";
import type { WorkflowBundleOption } from "@temporalio/worker";

import { applyCompensation, verifyApproval } from "./activities.js";
import type { NexusActivityRegistry } from "./activity-types.js";
import { NAMESPACE, TASK_QUEUES } from "./config.js";
import { NexusFailureInterceptor } from "./interceptors.js";

export interface TemporalWorkerOptions {
  readonly address?: string;
  readonly namespace?: string;
  /** Worker deployment/build identifier (EP-006 fallback doctrine). */
  readonly buildId?: string;
  /** Workflow task queues to poll (capability-separated). */
  readonly taskQueues: readonly string[];
  readonly extraActivities?: Partial<NexusActivityRegistry>;
  /** Directory or file the SDK bundles as workflow code. */
  readonly workflowsPath?: string;
  /** Pre-built workflow bundle (preferred for tests/production). */
  readonly workflowBundle?: WorkflowBundleOption;
}

const CORE_ACTIVITIES: Partial<NexusActivityRegistry> = {
  verifyApproval,
  applyCompensation,
};

/** Default workflow bundle entry (canonical type-name exports). */
export function defaultWorkflowsPath(): string {
  return path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    "workflows",
    "bundle.ts",
  );
}

export interface StartedWorker {
  /** One Worker per workflow queue, plus the shared activity Worker. */
  readonly workers: readonly Worker[];
  readonly connection: NativeConnection;
  shutdown(): Promise<void>;
}

/**
 * Create one Worker per workflow task queue plus one activity Worker on
 * the shared activity queue, all over the caller-owned connection.
 */
export async function createTemporalWorker(
  options: TemporalWorkerOptions,
  runtime: Runtime,
  connection: NativeConnection,
): Promise<StartedWorker> {
  void runtime;
  const namespace = options.namespace ?? NAMESPACE;
  const workflowsPath = options.workflowsPath ?? defaultWorkflowsPath();
  const workers: Worker[] = [];

  const workflowQueues = options.taskQueues.filter(
    (queue) => queue !== TASK_QUEUES.ACTIVITY,
  );
  for (const taskQueue of workflowQueues) {
    workers.push(
      await Worker.create({
        namespace,
        taskQueue,
        workflowsPath,
        ...(options.buildId === undefined ? {} : { buildId: options.buildId }),
        ...(options.workflowBundle === undefined
          ? {}
          : { workflowBundle: options.workflowBundle }),
        connection,
      }),
    );
  }

  const activities: Partial<NexusActivityRegistry> = {
    ...CORE_ACTIVITIES,
    ...options.extraActivities,
  };
  workers.push(
    await Worker.create({
      namespace,
      taskQueue: TASK_QUEUES.ACTIVITY,
      activities,
      // Classify NexusWorkflowError at the boundary so permanent SPEC-006
      // failures are never retried (SPEC-006 behavior 7).
      interceptors: {
        activity: [() => ({ inbound: new NexusFailureInterceptor() })],
      },
      ...(options.buildId === undefined ? {} : { buildId: options.buildId }),
      connection,
    }),
  );

  return {
    workers,
    connection,
    shutdown: async () => {
      for (const worker of workers) {
        await worker.shutdown();
      }
      // EP-005 owner doctrine: the CALLER owns the Runtime and
      // NativeConnection (see session.ts); the factory only stops its
      // workers and never closes a connection it does not own.
    },
  };
}

export { Runtime, NativeConnection };
