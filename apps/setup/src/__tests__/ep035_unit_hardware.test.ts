/**
 * EP-035 M1 HardwareProfiler provenance tests.
 *
 * Hardware facts carry provenance; "user says RTX GPU" is a
 * USER_DECLARED fact, never a detected GPU. Capability declarations are
 * claims, never certifications: CERTIFIED requires measured evidence and
 * a measured provenance.
 */

import { describe, expect, it } from "vitest";
import {
  HardwareCapabilityDeclaration,
  HardwareFact,
  HardwareProfile,
} from "../contracts/hardware";
import { ErrorCode, Spec006Error } from "../contracts/errors";

const CORRELATION = "00000000-0000-4000-8000-000000000001";

describe("ep035_unit_hardware", () => {
  it("preserves USER_DECLARED provenance without fabricating detection", () => {
    const fact = HardwareFact.parse({
      key: "gpu_model",
      value: "RTX 4090",
      provenance: "USER_DECLARED",
    });
    expect(fact.provenance).toBe("USER_DECLARED");
    expect(fact.toJSON().provenance).toBe("USER_DECLARED");
  });

  it("keeps HOST_OBSERVED distinct from USER_DECLARED", () => {
    const declared = HardwareFact.parse({
      key: "cpu_cores",
      value: 8,
      provenance: "USER_DECLARED",
    });
    const observed = HardwareFact.parse({
      key: "cpu_cores",
      value: 4,
      provenance: "HOST_OBSERVED",
      observed_at_unix_s: 1000,
    });
    expect(declared.provenance).not.toBe(observed.provenance);
  });

  it("rejects unknown fields and non-finite values", () => {
    expect(() =>
      HardwareFact.parse({
        key: "ram_bytes",
        value: 1,
        provenance: "HOST_OBSERVED",
        forged: true,
      }),
    ).toThrowError(Spec006Error);
    expect(() =>
      HardwareFact.parse({
        key: "ram_bytes",
        value: Number.NaN,
        provenance: "HOST_OBSERVED",
      }),
    ).toThrowError(Spec006Error);
  });

  it("capability declarations never certify from user-declared provenance", () => {
    expect(() =>
      HardwareCapabilityDeclaration.parse({
        capability_id: "local_llm",
        declaration_provenance: "USER_DECLARED",
        certification: "CERTIFIED",
        measured_evidence_id: "ev-1",
      }),
    ).toThrowError(Spec006Error);
  });

  it("CERTIFIED requires measured evidence AND measured provenance", () => {
    expect(() =>
      HardwareCapabilityDeclaration.parse({
        capability_id: "local_llm",
        declaration_provenance: "BENCHMARKED",
        certification: "CERTIFIED",
      }),
    ).toThrowError(Spec006Error);
    const certified = HardwareCapabilityDeclaration.parse({
      capability_id: "local_llm",
      declaration_provenance: "BENCHMARKED",
      certification: "CERTIFIED",
      measured_evidence_id: "bench-1",
    });
    expect(certified.certification).toBe("CERTIFIED");
  });

  it("observed facts alone never mint performance claims", () => {
    const profile = HardwareProfile.parse({
      facts: [
        {
          key: "cpu_cores",
          value: 16,
          provenance: "HOST_OBSERVED",
          observed_at_unix_s: 1000,
        },
        {
          key: "ram_bytes",
          value: 64,
          provenance: "HOST_OBSERVED",
          observed_at_unix_s: 1000,
        },
      ],
      capability_declarations: [],
      profiled_at_unix_s: 1001,
      correlation_id: CORRELATION,
    });
    expect(profile.capability_declarations.length).toBe(0);
    // No declaration may appear unless explicitly supplied.
    const wire = JSON.parse(JSON.stringify(profile));
    expect(wire.capability_declarations).toEqual([]);
  });

  it("round-trips hardware profile serialization", () => {
    const profile = HardwareProfile.parse({
      facts: [
        {
          key: "gpu_model",
          value: "RTX 4090",
          provenance: "USER_DECLARED",
        },
      ],
      capability_declarations: [],
      profiled_at_unix_s: 1001,
      correlation_id: CORRELATION,
    });
    const parsed = HardwareProfile.parse(JSON.parse(JSON.stringify(profile)));
    expect(parsed.facts[0]?.provenance).toBe("USER_DECLARED");
    expect(parsed.correlation_id).toBe(CORRELATION);
  });
});
