/**
 * Pure, deterministic compensation ledger (SPEC-006 behavior 8; ADR-010).
 *
 * Every EXTERNAL_EFFECT declares a compensation step; on failure or
 * cancel-with-compensate, steps run in REVERSE order, each exactly once,
 * keyed by idempotency. This module is pure and unit-tested; the engine
 * adapter executes the plan through activities.
 */

import type { ActivityId } from "@nexus/workflows";
import { WorkflowContractError } from "@nexus/workflows";

export interface CompensationEntry {
  readonly activityId: ActivityId;
  /** Canonical idempotency key of the ORIGINAL effect. */
  readonly effectIdempotencyKey: string;
  /** Compensation idempotency key (derived from the effect key). */
  readonly compensationKey: string;
  /** Execution order; compensations run in reverse order. */
  readonly order: number;
  readonly executed: boolean;
  readonly compensated: boolean;
}

export interface CompensationLedger {
  readonly entries: readonly CompensationEntry[];
}

export function emptyLedger(): CompensationLedger {
  return { entries: [] };
}

export function addCompensationEntry(
  ledger: CompensationLedger,
  entry: Omit<CompensationEntry, "executed" | "compensated">,
): CompensationLedger {
  if (ledger.entries.some((e) => e.compensationKey === entry.compensationKey)) {
    // Idempotent registration: same compensation key is a duplicate.
    return ledger;
  }
  return {
    entries: [
      ...ledger.entries,
      { ...entry, executed: false, compensated: false },
    ],
  };
}

/** Compensation execution plan: executed effects, reverse order. */
export function compensationPlan(
  ledger: CompensationLedger,
): readonly CompensationEntry[] {
  return ledger.entries
    .filter((e) => e.executed && !e.compensated)
    .sort((a, b) => b.order - a.order);
}

export function markEffectExecuted(
  ledger: CompensationLedger,
  compensationKey: string,
): CompensationLedger {
  return mapEntry(ledger, compensationKey, (e) => ({ ...e, executed: true }));
}

export function markLedgerCompensated(
  ledger: CompensationLedger,
  compensationKey: string,
): CompensationLedger {
  return mapEntry(ledger, compensationKey, (e) => ({
    ...e,
    compensated: true,
  }));
}

export function allCompensated(ledger: CompensationLedger): boolean {
  return ledger.entries.filter((e) => e.executed).every((e) => e.compensated);
}

/** Derive the compensation idempotency key from the effect key. */
export function compensationKeyFor(effectIdempotencyKey: string): string {
  return `comp:${effectIdempotencyKey}`;
}

function mapEntry(
  ledger: CompensationLedger,
  compensationKey: string,
  update: (entry: CompensationEntry) => CompensationEntry,
): CompensationLedger {
  let found = false;
  const entries = ledger.entries.map((e) => {
    if (e.compensationKey !== compensationKey) {
      return e;
    }
    found = true;
    return update(e);
  });
  if (!found) {
    throw new WorkflowContractError(
      `unknown compensation key ${compensationKey}`,
    );
  }
  return { entries };
}
