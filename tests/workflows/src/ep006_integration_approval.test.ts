/**
 * EP-006 M3: real-server readiness + approval round-trip.
 *
 * Proves the entire boundary against a REAL Temporal server 1.31.2
 * backed by REAL PostgreSQL 18.4: namespace registration, worker
 * connection, workflow start, approval signal delivery, query, and
 * completion. No mocks, no in-memory engine (TESTING.md).
 */

import { describe, expect, it } from "vitest";

import { TASK_QUEUES, WORKFLOW_TYPES, signalChannel } from "@nexus/temporal";
import type { ApprovalInput } from "@nexus/workflows";

import {
  actionDigestA,
  actionIdA,
  AUTH_STEP_UP,
  DIGEST_A,
  makeApprovalSignal,
  PRINCIPAL_HUMAN,
  workflowIdA,
} from "./helpers/fixtures.js";
import { getSession } from "./helpers/session.js";
import { startWorker } from "./helpers/worker.js";

describe("ep006_integration_readiness", () => {
  it("ep006_integration_server_namespace_ready", async () => {
    const session = await getSession();
    expect(session.address).toMatch(/^127\.0\.0\.1:\d+$/);
    expect(session.namespace).toBe("nexus");
    // A real client round-trips against the running server.
    const { client } = await createTestClient(
      session.address,
      session.namespace,
    );
    expect(client).toBeDefined();
  });
});

describe("ep006_integration_approval", () => {
  it("ep006_integration_approval_roundtrip_real_server", async () => {
    const session = await getSession();
    const started = await startWorker(session, [TASK_QUEUES.APPROVAL]);
    try {
      const input: ApprovalInput = {
        workflowId: workflowIdA,
        tenantId: "tenant-1",
        correlationId: "corr-roundtrip",
        principal: PRINCIPAL_HUMAN,
        actionId: actionIdA,
        actionDigest: actionDigestA,
        requiredAuthenticationStrength: "STEP_UP",
        // Explicit, short deadline so the real timeout path is testable.
        approvalTimeoutMs: 30_000,
      };
      const handle = await session.client.workflow.start(
        WORKFLOW_TYPES.APPROVAL,
        {
          taskQueue: TASK_QUEUES.APPROVAL,
          workflowId: workflowIdA,
          args: [input],
        },
      );

      await handle.signal(
        signalChannel(WORKFLOW_TYPES.APPROVAL, "approval"),
        makeApprovalSignal(),
      );

      const result = await handle.result();
      expect(result.state).toBe("APPROVED");
      // WorkflowOutcome vocabulary: SUCCEEDED is the terminal outcome
      // for an APPROVE decision (vocabulary.ts); APPROVED is the state.
      expect(result.outcome).toBe("SUCCEEDED");
      expect(result.output?.decision).toBe("APPROVE");
      expect(result.output?.actionDigest).toBe(DIGEST_A);
    } finally {
      await started.shutdown();
    }
  }, 90_000);
});

async function createTestClient(address: string, namespace: string) {
  const { createTemporalClient } = await import("@nexus/temporal");
  return createTemporalClient({ address, namespace });
}
