/**
 * Temporal workflow client factory (ADR-010; SPEC-023).
 *
 * Start workflows by their canonical type names, signal and query them by
 * workflowId. The client is provider code; all workflow input/output flows
 * through the @nexus/workflows contracts.
 */

import { Client, Connection } from "@temporalio/client";

import { NAMESPACE } from "./config.js";
import { WORKFLOW_TYPES } from "./config.js";
import type { WorkflowTypeName } from "./config.js";

export interface TemporalClientOptions {
  readonly address?: string;
  readonly namespace?: string;
}

export async function createTemporalClient(
  options: TemporalClientOptions = {},
): Promise<{ client: Client; connection: Connection }> {
  const connection = await Connection.connect({
    address: options.address ?? "localhost:7233",
  });
  const client = new Client({
    connection,
    namespace: options.namespace ?? NAMESPACE,
  });
  return { client, connection };
}

export { WORKFLOW_TYPES };
export type { WorkflowTypeName };
