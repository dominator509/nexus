import { describe, expect, it } from "vitest";

import {
  dedupeSignals,
  isIdempotentDuplicate,
  signalKey,
  validateSignal,
} from "../signals.js";
import {
  makeApprovalSignal,
  signalIdA,
  signalIdB,
  workflowIdA,
  workflowIdB,
} from "./helpers/fixtures.js";

describe("ep006_unit_signals_idempotent", () => {
  it("ep006_unit_signals_same_signal_id_is_duplicate", () => {
    const first = makeApprovalSignal();
    const redelivery = makeApprovalSignal({
      decidedAt: "2026-08-13T01:00:00Z",
    });
    expect(isIdempotentDuplicate(first, redelivery)).toBe(true);
    expect(signalKey(first)).toBe(signalKey(redelivery));
  });

  it("ep006_unit_signals_different_signal_id_not_duplicate", () => {
    const first = makeApprovalSignal();
    const second = makeApprovalSignal({ signalId: signalIdB });
    expect(isIdempotentDuplicate(first, second)).toBe(false);
  });

  it("ep006_unit_signals_different_workflow_not_duplicate", () => {
    const first = makeApprovalSignal();
    const otherWorkflow = makeApprovalSignal({ workflowId: workflowIdB });
    expect(isIdempotentDuplicate(first, otherWorkflow)).toBe(false);
  });

  it("ep006_unit_signals_different_type_not_duplicate", () => {
    // Same signalId but a different signal type is a different logical
    // effect; the key includes the type.
    const approval = makeApprovalSignal();
    const cancel = {
      signalType: "CANCEL" as const,
      signalId: signalIdA,
      workflowId: workflowIdA,
      requestedAt: "2026-08-13T00:00:00Z",
    };
    expect(isIdempotentDuplicate(approval, cancel)).toBe(false);
  });

  it("ep006_unit_signals_key_is_stable_and_pure", () => {
    const a = makeApprovalSignal();
    const b = makeApprovalSignal({ comment: "different comment" });
    // Payload differences do not change the dedup key.
    expect(signalKey(a)).toBe(signalKey(b));
  });

  it("ep006_unit_signals_dedupe_keeps_first_delivery", () => {
    const first = makeApprovalSignal();
    const redelivery = makeApprovalSignal({
      decidedAt: "2026-08-13T09:00:00Z",
    });
    const different = makeApprovalSignal({ signalId: signalIdB });
    const deduped = dedupeSignals([first, redelivery, different]);
    expect(deduped).toHaveLength(2);
    expect(deduped[0]).toBe(first);
    expect(deduped[1]).toBe(different);
  });

  it("ep006_unit_signals_dedupe_validates_typed_input", () => {
    const raw = makeApprovalSignal();
    const parsed = validateSignal(JSON.parse(JSON.stringify(raw)));
    const deduped = dedupeSignals([parsed, parsed]);
    expect(deduped).toHaveLength(1);
  });
});
