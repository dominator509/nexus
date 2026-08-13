import { describe, expect, it } from "vitest";

import { WorkflowContractError } from "@nexus/workflows";

import {
  addCompensationEntry,
  allCompensated,
  compensationKeyFor,
  compensationPlan,
  emptyLedger,
  markEffectExecuted,
  markLedgerCompensated,
} from "../state/compensation.js";
import { workflowIdA } from "./helpers/fixtures.js";

const KEY_A = `${workflowIdA}:act-a:1`;
const KEY_B = `${workflowIdA}:act-b:1`;

function entry(key: string, order: number) {
  return {
    activityId: key as unknown as import("@nexus/workflows").ActivityId,
    effectIdempotencyKey: key,
    compensationKey: compensationKeyFor(key),
    order,
  };
}

describe("ep006_unit_compensation", () => {
  it("ep006_unit_compensation_key_derivation", () => {
    expect(compensationKeyFor(KEY_A)).toBe(`comp:${KEY_A}`);
  });

  it("ep006_unit_compensation_registration_idempotent", () => {
    let ledger = emptyLedger();
    ledger = addCompensationEntry(ledger, entry(KEY_A, 1));
    ledger = addCompensationEntry(ledger, entry(KEY_A, 1));
    expect(ledger.entries).toHaveLength(1);
  });

  it("ep006_unit_compensation_plan_reverse_order_executed_only", () => {
    let ledger = emptyLedger();
    ledger = addCompensationEntry(ledger, entry(KEY_A, 1));
    ledger = addCompensationEntry(ledger, entry(KEY_B, 2));
    // Nothing executed yet: empty plan.
    expect(compensationPlan(ledger)).toHaveLength(0);
    ledger = markEffectExecuted(ledger, compensationKeyFor(KEY_A));
    ledger = markEffectExecuted(ledger, compensationKeyFor(KEY_B));
    const plan = compensationPlan(ledger);
    // Reverse order: B (order 2) before A (order 1).
    expect(plan.map((e) => e.effectIdempotencyKey)).toEqual([KEY_B, KEY_A]);
  });

  it("ep006_unit_compensation_compensated_steps_excluded", () => {
    let ledger = emptyLedger();
    ledger = addCompensationEntry(ledger, entry(KEY_A, 1));
    ledger = addCompensationEntry(ledger, entry(KEY_B, 2));
    ledger = markEffectExecuted(ledger, compensationKeyFor(KEY_A));
    ledger = markEffectExecuted(ledger, compensationKeyFor(KEY_B));
    ledger = markLedgerCompensated(ledger, compensationKeyFor(KEY_A));
    expect(compensationPlan(ledger).map((e) => e.effectIdempotencyKey)).toEqual(
      [KEY_B],
    );
    expect(allCompensated(ledger)).toBe(false);
    ledger = markLedgerCompensated(ledger, compensationKeyFor(KEY_B));
    expect(allCompensated(ledger)).toBe(true);
  });

  it("ep006_unit_compensation_unknown_key_throws", () => {
    const ledger = emptyLedger();
    expect(() => markEffectExecuted(ledger, compensationKeyFor(KEY_A))).toThrow(
      WorkflowContractError,
    );
  });

  it("ep006_unit_compensation_no_executed_effects_all_compensated", () => {
    expect(allCompensated(emptyLedger())).toBe(true);
  });
});
