/**
 * EP-006 M3: deterministic teardown as a hard invariant.
 *
 * 1. ep006_integration_teardown_dispose_leaves_no_resources - starts an
 *    INDEPENDENT real stack (not the shared session), runs one minimal
 *    REAL workflow interaction, then explicitly `await stack.dispose()`
 *    from finally. After disposal it queries real Docker/process state
 *    and proves the server container, postgres container, any
 *    admin-tools/one-shot container, the network, the volumes, and the
 *    server child process are all gone. The worker runs in-process, so
 *    "no worker process" is proven by the awaited shutdown completing
 *    without a run() rejection (any rejection throws here).
 * 2. ep006_integration_teardown_forced_failure_is_surfaced - at the
 *    narrowest safe layer, forces a REAL docker cleanup failure (volume
 *    in use by a container dispose was not told about) and proves the
 *    error is surfaced (never swallowed) while the remaining cleanup
 *    steps still run.
 *
 * NOTE on suite ordering: vitest does NOT guarantee alphabetical file
 * order, so this file never disposes the SHARED session. The shared
 * session is disposed by the suite-level globalTeardown
 * (vitest.config.ts / global-teardown.ts) - an explicit async dispose
 * that runs after every test file, independent of file order. The
 * process-exit hook in helpers/session.ts is only the last-resort
 * emergency net. This file's own stack proves the primary per-stack
 * try/finally dispose pattern end-to-end.
 */

import { describe, expect, it } from "vitest";

import { TASK_QUEUES, WORKFLOW_TYPES, signalChannel } from "@nexus/temporal";
import type { ApprovalInput } from "@nexus/workflows";
import {
  parseActionDigest,
  parseSignalId,
  parseWorkflowId,
  type WorkflowId,
} from "@nexus/workflows";
import { Client, Connection } from "@temporalio/client";
import { NativeConnection } from "@temporalio/worker";

import {
  actionDigestA,
  actionIdA,
  AUTH_STEP_UP,
  makeApprovalSignal,
  PRINCIPAL_HUMAN,
} from "./helpers/fixtures.js";
import { getSession } from "./helpers/session.js";
import type { TestSession } from "./helpers/session.js";
import {
  disposeStackResources,
  POSTGRES_IMAGE,
  POSTGRES_DIGEST,
  runDocker,
  stackSuffix,
  startTemporalStack,
  type StackResources,
} from "./helpers/stack.js";
import { startWorker } from "./helpers/worker.js";

const TD_WID = "0193a1f2-0000-7000-8000-000000000401";
const TD_SIGNAL = "0193a1f2-0000-7000-8000-000000000402";

function approvalInput(workflowId: WorkflowId): ApprovalInput {
  return {
    workflowId,
    tenantId: "tenant-teardown",
    correlationId: "corr-teardown",
    principal: PRINCIPAL_HUMAN,
    actionId: actionIdA,
    actionDigest: actionDigestA,
    requiredAuthenticationStrength: "STEP_UP",
    approvalTimeoutMs: 30_000,
  };
}

function queryChannelName(workflowType: string, kind: string): string {
  return `${workflowType}.query.${kind}`;
}

function containerExists(name: string): boolean {
  const out = runDocker([
    "ps",
    "-a",
    "--filter",
    `name=${name}`,
    "--format",
    "{{.Names}}",
  ]);
  return out
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .includes(name);
}

function networkExists(name: string): boolean {
  const out = runDocker([
    "network",
    "ls",
    "--filter",
    `name=${name}`,
    "--format",
    "{{.Name}}",
  ]);
  return out
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .includes(name);
}

function volumeExists(name: string): boolean {
  const out = runDocker([
    "volume",
    "ls",
    "--filter",
    `name=${name}`,
    "--format",
    "{{.Name}}",
  ]);
  return out
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .includes(name);
}

function containerPid(name: string): number {
  return Number(
    runDocker(["inspect", "--format", "{{.State.Pid}}", name]).trim(),
  );
}

function processAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

describe("ep006_integration_teardown", () => {
  it("ep006_integration_teardown_dispose_leaves_no_resources", async () => {
    const session = await getSession();
    // An INDEPENDENT stack owned by this test (its own containers,
    // network, volume, namespace). The shared session's Runtime is
    // reused (Runtime is a process singleton); the connection is THIS
    // test's own and closed by THIS test (EP-005 owner doctrine).
    const stack = await startTemporalStack();
    const connection = await NativeConnection.connect({
      address: stack.address,
    });
    const sdkConnection = await Connection.connect({
      address: stack.address,
    });
    const client = new Client({
      connection: sdkConnection,
      namespace: stack.namespace,
    });
    const ownSession = {
      stack,
      address: stack.address,
      namespace: stack.namespace,
      runtime: session.runtime,
      connection,
      client,
    } as unknown as TestSession;

    const workflowId = parseWorkflowId(TD_WID);
    const serverPid = containerPid(stack.serverContainer);
    expect(serverPid).toBeGreaterThan(0);

    let worker: Awaited<ReturnType<typeof startWorker>> | undefined;
    try {
      worker = await startWorker(ownSession, [TASK_QUEUES.APPROVAL]);
      const handle = await client.workflow.start(WORKFLOW_TYPES.APPROVAL, {
        taskQueue: TASK_QUEUES.APPROVAL,
        workflowId,
        args: [approvalInput(workflowId)],
      });
      await handle.signal(
        signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
        makeApprovalSignal({
          signalId: parseSignalId(TD_SIGNAL),
          workflowId,
        }),
      );
      const result = await handle.result();
      expect(result.state).toBe("APPROVED");
      expect(result.outcome).toBe("SUCCEEDED");
    } finally {
      if (worker !== undefined) {
        // Explicit, awaited worker shutdown (in-process worker: a run()
        // rejection would throw here and fail the test).
        await worker.shutdown();
      }
      // Caller-owned connections closed by the caller (EP-005).
      await connection.close();
      await sdkConnection.close();
      // PRIMARY teardown path: explicit async dispose from finally.
      await stack.dispose();
    }

    // ---- Post-dispose REAL proofs (docker + process state) ----
    expect(containerExists(stack.serverContainer)).toBe(false);
    expect(containerExists(stack.postgresContainer)).toBe(false);
    // Network removal succeeding is itself the proof that no container
    // (including any admin-tools / one-shot container) remained
    // attached: docker refuses to remove a network with active
    // endpoints. The sweep in dispose() removed any such container
    // before the network removal, and if that sweep failed the network
    // removal would have failed too and surfaced here.
    expect(networkExists(stack.network)).toBe(false);
    for (const volume of stack.volumes) {
      expect(volumeExists(volume)).toBe(false);
    }
    // The temporal-server start process is the server container's main
    // process; removing the container terminated it. Prove the exact
    // host PID observed before disposal no longer exists.
    expect(processAlive(serverPid)).toBe(false);
  }, 120_000);

  it("ep006_integration_teardown_forced_failure_is_surfaced", async () => {
    const suffix = stackSuffix();
    const holder = `nexus-ep006-failholder-${suffix}`;
    const inUseVolume = `nexus-ep006-failvol-inuse-${suffix}`;
    const cleanVolume = `nexus-ep006-failvol-clean-${suffix}`;
    const missing = `nexus-ep006-absent-${suffix}`;
    try {
      // REAL resources: a volume mounted by a container that dispose is
      // NOT told about (lost-track scenario) + a clean volume that must
      // still be removed after the failure.
      runDocker(["volume", "create", inUseVolume]);
      runDocker(["volume", "create", cleanVolume]);
      runDocker([
        "run",
        "-d",
        "--name",
        holder,
        "-v",
        `${inUseVolume}:/data`,
        "--entrypoint",
        "/bin/true",
        `${POSTGRES_IMAGE}@${POSTGRES_DIGEST}`,
      ]);

      const resources: StackResources = {
        postgresContainer: missing,
        serverContainer: missing,
        network: missing,
        volumes: [inUseVolume, cleanVolume],
      };

      // The in-use volume removal must FAIL with a real docker error,
      // and that failure must be surfaced (rejected promise) rather
      // than swallowed by the teardown routine.
      await expect(disposeStackResources(resources)).rejects.toThrow(
        /volume is in use/,
      );

      // Remaining cleanup steps still ran after the failure: the clean
      // volume was removed even though the in-use one failed.
      expect(volumeExists(cleanVolume)).toBe(false);
      expect(volumeExists(inUseVolume)).toBe(true);
    } finally {
      // Test-owned cleanup of the REAL leftover resources (not part of
      // the teardown-under-test): must succeed.
      runDocker(["rm", "-f", holder]);
      runDocker(["volume", "rm", inUseVolume]);
      if (volumeExists(cleanVolume)) {
        runDocker(["volume", "rm", cleanVolume]);
      }
    }
  }, 60_000);
});
