/**
 * EP-033 M5 LF-005 cross-device continuity evidence (current-run bound).
 *
 * The journey logic is proven in ep033_a11y_unit.test.ts; this test exists
 * so the M5 gate can observe a FRESH LF-005 evidence file written through
 * the same transform pipeline the browser scan uses (plain `node dist`
 * cannot resolve workspace TS sources). The gate rejects stale evidence by
 * binding every proof to the current run_id.
 */

import { describe, expect, it } from "vitest";
import { runLf005Journey, writeEvidence, runId } from "../lf005.js";
import { existsSync, readFileSync } from "node:fs";

describe("ep033_lf005_evidence", () => {
  it("writes current-run LF-005 evidence that the gate can observe", () => {
    const run = runId();
    const evidence = runLf005Journey(run);
    const path = writeEvidence(evidence);

    expect(existsSync(path)).toBe(true);
    const onDisk = JSON.parse(readFileSync(path, "utf8")) as {
      run_id: string;
      node: string;
      milestone: string;
      journey: {
        web_dashboard_continue: { objective_bound: boolean };
        mobile_approval: { satisfied: boolean };
        final_artifact_same_graph: {
          objective_ids_consistent: boolean;
          correlation_consistent: boolean;
        };
      };
      authority_distinctions: { executed_only_after_dispatch: boolean };
    };

    expect(onDisk.run_id).toBe(run);
    expect(onDisk.node).toBe("EP-033");
    expect(onDisk.milestone).toBe("M5");
    expect(onDisk.journey.web_dashboard_continue.objective_bound).toBe(true);
    expect(onDisk.journey.mobile_approval.satisfied).toBe(true);
    expect(
      onDisk.journey.final_artifact_same_graph.objective_ids_consistent,
    ).toBe(true);
    expect(
      onDisk.journey.final_artifact_same_graph.correlation_consistent,
    ).toBe(true);
    expect(onDisk.authority_distinctions.executed_only_after_dispatch).toBe(
      true,
    );
    console.log(`[ep033_lf005_evidence] run=${run} evidence=${path}`);
  });
});
