/**
 * EP-006 M4: duplicate-request failure class (execplan M4 content 1).
 *
 * Duplicate signals/requests must collapse on signalKey without double
 * effect; the dedup helpers must be total (never crash on malformed
 * input) and the key must distinguish every logical signal.
 */

import { describe, expect, it } from "vitest";

import {
  dedupeSignals,
  isIdempotentDuplicate,
  signalKey,
  validateApprovalSignal,
} from "../index.js";
import { makeApprovalSignal, signalIdA, signalIdB } from "./helpers/fixtures.js";

describe("ep006_failure_duplicate_request", () => {
  it("ep006_failure_duplicate_signal_same_key", () => {
    const first = makeApprovalSignal();
    const second = makeApprovalSignal();
    expect(signalKey(first)).toBe(signalKey(second));
    expect(isIdempotentDuplicate(first, second)).toBe(true);
  });

  it("ep006_failure_duplicate_distinct_signal_id_distinct_key", () => {
    const first = makeApprovalSignal({ signalId: signalIdA });
    const second = makeApprovalSignal({ signalId: signalIdB });
    expect(signalKey(first)).not.toBe(signalKey(second));
    expect(isIdempotentDuplicate(first, second)).toBe(false);
  });

  it("ep006_failure_duplicate_dedupes_to_single_observed", () => {
    const approval = makeApprovalSignal();
    const duplicates = [
      validateApprovalSignal({ ...approval }),
      validateApprovalSignal({ ...approval }),
      validateApprovalSignal({ ...approval }),
    ];
    const deduped = dedupeSignals(duplicates);
    expect(deduped).toHaveLength(1);
    expect(deduped[0]?.signalId).toBe(signalIdA);
  });

  it("ep006_failure_duplicate_same_id_different_payload_same_key", () => {
    // signalKey is (workflowId, type, signalId): two redeliveries of the
    // SAME logical signal with a payload field changed are still one key.
    const first = makeApprovalSignal({ comment: "v1" });
    const second = makeApprovalSignal({ comment: "v2" });
    expect(signalKey(first)).toBe(signalKey(second));
    expect(isIdempotentDuplicate(first, second)).toBe(true);
  });

  it("ep006_failure_duplicate_dedupes_totality", () => {
    // dedupeSignals must be total: an empty list is a no-op.
    expect(dedupeSignals([])).toEqual([]);
  });
});
