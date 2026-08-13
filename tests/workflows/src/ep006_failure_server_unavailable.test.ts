/**
 * EP-006 M4: unavailable-dependency failure class (execplan M4 content
 * 1-2) on a REAL Temporal server.
 *
 * Real mechanism: the ephemeral Temporal server container is terminated
 * with docker rm -f while a workflow is open. The client must fail
 * closed with a structured transport/service error (never hang, never
 * silently succeed), and the stack must still dispose cleanly.
 *
 * Uses an INDEPENDENT stack (never the shared session stack) so the
 * termination cannot poison later tests. No worker is started: the
 * failure under test is the client's dependency on the server.
 */

import { describe, expect, it } from "vitest";

import { TASK_QUEUES, WORKFLOW_TYPES, signalChannel } from "@nexus/temporal";
import type { ApprovalInput } from "@nexus/workflows";
import {
  parseActionDigest,
  parseSignalId,
  parseWorkflowId,
} from "@nexus/workflows";
import { Client, Connection } from "@temporalio/client";

import {
  actionDigestA,
  actionIdA,
  AUTH_STEP_UP,
  makeApprovalSignal,
  PRINCIPAL_HUMAN,
} from "./helpers/fixtures.js";
import { getSession } from "./helpers/session.js";
import { startTemporalStack, runDocker } from "./helpers/stack.js";
import {
  containerExists,
  networkExists,
  volumeExists,
} from "./helpers/docker-proofs.js";

const WID_UNAVAILABLE = "0193a1f2-0000-7000-8000-000000000501";

function approvalInput(
  workflowId: ReturnType<typeof parseWorkflowId>,
): ApprovalInput {
  return {
    workflowId,
    tenantId: "tenant-m4",
    correlationId: "corr-unavailable",
    principal: PRINCIPAL_HUMAN,
    actionId: actionIdA,
    actionDigest: actionDigestA,
    requiredAuthenticationStrength: "STEP_UP",
    approvalTimeoutMs: 30_000,
  };
}

describe("ep006_failure_server_unavailable", () => {
  it("ep006_failure_server_unavailable_fails_closed", async () => {
    const session = await getSession(); // shared Runtime only
    const stack = await startTemporalStack();
    const sdkConnection = await Connection.connect({ address: stack.address });
    const client = new Client({
      connection: sdkConnection,
      namespace: stack.namespace,
    });
    const workflowId = parseWorkflowId(WID_UNAVAILABLE);
    try {
      // The workflow starts successfully while the server is up.
      const handle = await client.workflow.start(WORKFLOW_TYPES.APPROVAL, {
        taskQueue: TASK_QUEUES.APPROVAL,
        workflowId,
        args: [approvalInput(workflowId)],
      });
      expect(handle.workflowId).toBe(workflowId);

      // REAL failure mechanism: terminate the server container.
      runDocker(["rm", "-f", stack.serverContainer]);

      // The client must fail closed - a structured transport/service
      // error, never a silent success and never a hang.
      let failed = false;
      try {
        await handle.signal(
          signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
          makeApprovalSignal({
            signalId: parseSignalId(WID_UNAVAILABLE),
            workflowId,
          }),
        );
      } catch (error) {
        failed = true;
        // The error is the SDK's typed transport/service error; assert
        // it carries a message and a name (structured, not a bare string).
        expect(error).toBeInstanceOf(Error);
        expect((error as Error).name.length).toBeGreaterThan(0);
        expect(String((error as Error).message).length).toBeGreaterThan(0);
      }
      expect(failed).toBe(true);
    } finally {
      // Caller-owned connection closed by the caller (EP-005).
      await sdkConnection.close();
      // Explicit disposal: the terminated server container is already
      // gone; postgres, the network, and volumes must still be removed.
      await stack.dispose();
    }

    // Post-dispose proofs: no EP-006 resources from this stack remain.
    expect(containerExists(stack.serverContainer)).toBe(false);
    expect(containerExists(stack.postgresContainer)).toBe(false);
    expect(networkExists(stack.network)).toBe(false);
    for (const volume of stack.volumes) {
      expect(volumeExists(volume)).toBe(false);
    }
  }, 120_000);
});
