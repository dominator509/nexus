/**
 * EP-017 agent workflow vocabulary (SPEC-010; ADR-024).
 *
 * SPEC-010 locks the canonical terms Agent Registry, Agent Adapter,
 * Agent Capability, Objective, AgentTask, Delegation, Artifact, Agent
 * Skills, Skill Trust, and Skill Factory. This module adds the
 * EP-017-owned durable workflow vocabulary: the agent workflow kinds
 * and operation kinds the agent plane's durable workflows are built
 * from.
 *
 * Every enum here is vocabulary locked: unknown values are rejected at
 * parse time, and a new synonym requires an ADR and schema update
 * (ADR-024).
 */

import { WorkflowContractError } from "../errors.js";

export interface AgentsVocabulary<T extends string> {
  readonly name: string;
  readonly values: readonly T[];
  is(value: unknown): value is T;
  parse(value: unknown, context?: string): T;
}

export function defineAgentsVocabulary<T extends string>(
  name: string,
  values: readonly T[],
): AgentsVocabulary<T> {
  const set = new Set<string>(values);
  return {
    name,
    values,
    is(value: unknown): value is T {
      return typeof value === "string" && set.has(value);
    },
    parse(value: unknown, context = name): T {
      if (typeof value === "string" && set.has(value)) {
        return value as T;
      }
      throw new WorkflowContractError(
        `invalid ${context}: ${JSON.stringify(value)}; expected one of ${values.join(", ")}`,
      );
    },
  };
}

/**
 * Agent workflow kinds (SPEC-010; ADR-024). Each kind is a durable,
 * audited workflow over the agent plane. Kinds are EP-017-owned; they
 * are distinct from the EP-006 workflow kinds and are never mixed with
 * them at the registry boundary.
 */
export const agentWorkflowKind = defineAgentsVocabulary("AgentWorkflowKind", [
  "TASK_ASSIGNMENT",
  "DELEGATION",
  "ARTIFACT_EXCHANGE",
  "REVIEW_LOOP",
  "CANCELLATION",
  "BUDGET_ENFORCEMENT",
] as const);
export type AgentWorkflowKind = (typeof agentWorkflowKind.values)[number];

/**
 * Agent operation kinds: the durable activity-level operations the
 * agent workflows schedule. Each maps to a bounded, idempotent,
 * error-classified activity (SPEC-006 behavior 7). Kinds are
 * EP-017-owned.
 */
export const agentOperationKind = defineAgentsVocabulary("AgentOperationKind", [
  "SELECT_CANDIDATES",
  "ASSIGN_AGENT",
  "START_SESSION",
  "RECORD_DELEGATION",
  "ATTACH_ARTIFACT",
  "SUBMIT_REVIEW",
  "CANCEL_TASK",
  "REVOKE_DELEGATION",
  "CONSUME_BUDGET",
  "PUBLISH_RESULT",
] as const);
export type AgentOperationKind = (typeof agentOperationKind.values)[number];

/**
 * Agent workflow lifecycle states. Terminal outcomes mirror SPEC-006
 * ActionLifecycle; durable engine states CANCELLED and TIMED_OUT are
 * explicit (SPEC-023 behavior 5).
 */
export const agentWorkflowState = defineAgentsVocabulary("AgentWorkflowState", [
  "REQUESTED",
  "ASSIGNED",
  "RUNNING",
  "WAITING_INPUT",
  "REVIEWING",
  "PAUSED",
  "SUCCEEDED",
  "FAILED",
  "CANCELLED",
] as const);
export type AgentWorkflowState = (typeof agentWorkflowState.values)[number];

/**
 * Review verdict in the Codex-implement / Claude-review loop
 * (SPEC-010 behavior 4; ADR-024). APPROVE completes the loop;
 * REQUEST_CHANGES returns it to the implementer (bounded iterations);
 * REJECT fails the task.
 */
export const reviewVerdict = defineAgentsVocabulary("ReviewVerdict", [
  "APPROVE",
  "REQUEST_CHANGES",
  "REJECT",
] as const);
export type ReviewVerdict = (typeof reviewVerdict.values)[number];

/**
 * Artifact disposition in the artifact exchange (SPEC-010; ADR-024).
 * Artifacts are immutable by content hash; SUPERSEDED and REVOKED are
 * lineage states, never content mutations.
 */
export const artifactDisposition = defineAgentsVocabulary(
  "ArtifactDisposition",
  ["ATTACHED", "SUPERSEDED", "REVOKED"] as const,
);
export type ArtifactDisposition = (typeof artifactDisposition.values)[number];
