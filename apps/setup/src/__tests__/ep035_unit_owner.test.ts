/**
 * EP-035 M1 OwnerBootstrap tests.
 *
 * Owner bootstrap is security-critical: OWNER_DETAILS_PROVIDED !=
 * OWNER_IDENTITY_VERIFIED != OWNER_PRINCIPAL_CREATED != OWNER_AUTHORIZED.
 * A client-side `isOwner` flag is rejected (deny-unknown). First-owner
 * initialization is deterministic: replay is idempotent, competition is
 * CONFLICT.
 */

import { describe, expect, it } from "vitest";
import {
  FirstOwnerKnown,
  OwnerBootstrapRequest,
  OwnerBootstrapStateRecord,
  resolveFirstOwnerRequest,
} from "../contracts/owner";
import { ErrorCode, Spec006Error } from "../contracts/errors";

const CORRELATION = "00000000-0000-4000-8000-000000000001";
const PRINCIPAL = "00000000-0000-4000-8000-000000000002";

function sampleRequest(overrides: Record<string, unknown> = {}): unknown {
  return {
    owner_name: "Alice Owner",
    owner_email: "alice@example.com",
    correlation_id: CORRELATION,
    idempotency_key: "bootstrap-1",
    ...overrides,
  };
}

describe("ep035_unit_owner", () => {
  it("parses a valid bootstrap request", () => {
    const request = OwnerBootstrapRequest.parse(sampleRequest());
    expect(request.owner_email).toBe("alice@example.com");
  });

  it("rejects a client-side isOwner authority flag", () => {
    expect(() =>
      OwnerBootstrapRequest.parse(sampleRequest({ isOwner: true })),
    ).toThrowError(Spec006Error);
  });

  it("rejects unknown fields and missing required values", () => {
    expect(() =>
      OwnerBootstrapRequest.parse(sampleRequest({ forged: 1 })),
    ).toThrowError(Spec006Error);
    const { owner_name: _name, ...missingName } = sampleRequest() as Record<
      string,
      unknown
    >;
    expect(() => OwnerBootstrapRequest.parse(missingName)).toThrowError(
      Spec006Error,
    );
  });

  it("models the owner ladder as distinct states", () => {
    const request = OwnerBootstrapRequest.parse(sampleRequest());
    const provided = OwnerBootstrapStateRecord.detailsProvided(request, 1000);
    expect(provided.state).toBe("OWNER_DETAILS_PROVIDED");
    const verified = provided.advance(
      "OWNER_IDENTITY_VERIFIED",
      1001,
      undefined,
      "recovery-kit",
    );
    expect(verified.state).toBe("OWNER_IDENTITY_VERIFIED");
    const created = verified.advance(
      "OWNER_PRINCIPAL_CREATED",
      1002,
      PRINCIPAL,
    );
    expect(created.state).toBe("OWNER_PRINCIPAL_CREATED");
    const authorized = created.advance("OWNER_AUTHORIZED", 1003, PRINCIPAL);
    expect(authorized.state).toBe("OWNER_AUTHORIZED");
  });

  it("rejects identity verification without a method", () => {
    const request = OwnerBootstrapRequest.parse(sampleRequest());
    const provided = OwnerBootstrapStateRecord.detailsProvided(request, 1000);
    try {
      provided.advance("OWNER_IDENTITY_VERIFIED", 1001);
      throw new Error("expected rejection");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Verification);
    }
  });

  it("rejects the DETAILS -> PRINCIPAL leap", () => {
    const request = OwnerBootstrapRequest.parse(sampleRequest());
    const provided = OwnerBootstrapStateRecord.detailsProvided(request, 1000);
    try {
      provided.advance("OWNER_PRINCIPAL_CREATED", 1001, PRINCIPAL);
      throw new Error("expected rejection");
    } catch (err) {
      expect((err as Spec006Error).code).toBe(ErrorCode.Policy);
    }
  });

  it("first-owner initialize returns INITIALIZED for the first request", () => {
    const request = OwnerBootstrapRequest.parse(sampleRequest());
    const result = resolveFirstOwnerRequest(undefined, request, PRINCIPAL);
    expect(result.kind).toBe("INITIALIZED");
    if (result.kind === "INITIALIZED") {
      expect(result.principal_id).toBe(PRINCIPAL);
    }
  });

  it("replaying the same bootstrap request is idempotent", () => {
    const request = OwnerBootstrapRequest.parse(sampleRequest());
    const known = new FirstOwnerKnown("bootstrap-1", PRINCIPAL);
    const replay = resolveFirstOwnerRequest(known, request, PRINCIPAL);
    expect(replay.kind).toBe("ALREADY_INITIALIZED");
    if (replay.kind === "ALREADY_INITIALIZED") {
      expect(replay.principal_id).toBe(PRINCIPAL);
    }
  });

  it("a competing first-owner request is CONFLICT, never a second owner", () => {
    const request = OwnerBootstrapRequest.parse(
      sampleRequest({ idempotency_key: "bootstrap-2" }),
    );
    const known = new FirstOwnerKnown("bootstrap-1", PRINCIPAL);
    expect(resolveFirstOwnerRequest(known, request, PRINCIPAL).kind).toBe(
      "CONFLICT",
    );
  });

  it("round-trips state serialization with deny-unknown", () => {
    const request = OwnerBootstrapRequest.parse(sampleRequest());
    const created = OwnerBootstrapStateRecord.detailsProvided(request, 1000)
      .advance("OWNER_IDENTITY_VERIFIED", 1001, undefined, "recovery-kit")
      .advance("OWNER_PRINCIPAL_CREATED", 1002, PRINCIPAL);
    const parsed = OwnerBootstrapStateRecord.parse(
      JSON.parse(JSON.stringify(created)),
    );
    expect(parsed.state).toBe("OWNER_PRINCIPAL_CREATED");
    expect(parsed.principal_id).toBe(PRINCIPAL);
    expect(() =>
      OwnerBootstrapStateRecord.parse({
        ...JSON.parse(JSON.stringify(created)),
        forged: true,
      }),
    ).toThrowError(Spec006Error);
  });
});
