/**
 * EP-035 M3 integration: real durable first-owner bootstrap.
 *
 * Proves the first-owner concurrency boundary against REAL PostgreSQL
 * 18.4: exactly one canonical initial owner, deterministic replay via
 * idempotency key, competing second owner -> Conflict, and exact-target
 * readback through the durable store.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { randomUUID } from "node:crypto";
import {
  OwnerBootstrapRequest,
  OwnerBootstrapStateRecord,
  ErrorCode,
} from "@nexus/setup";
import {
  OwnerBootstrapStore,
  derivePrincipalId,
} from "../../stores/owner-bootstrap.store.js";
import {
  freshDb,
  pgVersion,
  startStack,
  stopStack,
  type TestStack,
} from "./harness.js";

describe("ep035_integration_owner_bootstrap", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  function request(idempotencyKey: string): OwnerBootstrapRequest {
    return OwnerBootstrapRequest.parse({
      owner_name: "Dominic",
      owner_email: `owner-${randomUUID().slice(0, 8)}@nexus.test`,
      correlation_id: randomUUID(),
      idempotency_key: idempotencyKey,
    });
  }

  it("records the actual runtime postgres version", () => {
    const version = pgVersion(stack);
    expect(version).toMatch(/^18\./);
  });

  it("initializes exactly one canonical first owner", async () => {
    const db = await freshDb(stack);
    const store = new OwnerBootstrapStore(db);
    const key = `first-owner-${randomUUID().slice(0, 8)}`;
    const req = request(key);
    const principal = derivePrincipalId(req);

    const first = await store.initialize(req, principal, 1_700_000_000);
    expect(first).toEqual({ kind: "INITIALIZED", principal_id: principal });

    // Exact-target readback: the durable row is the source of truth.
    const owner = await store.readOwnerById(principal);
    expect(owner).toBeDefined();
    expect(owner!.owner_id).toBe(principal);
    expect(owner!.idempotency_key).toBe(key);
    expect(owner!.state).toBe("OWNER_PRINCIPAL_CREATED");

    await db.close();
  });

  it("replays the same bootstrap request deterministically (idempotent)", async () => {
    const db = await freshDb(stack);
    const store = new OwnerBootstrapStore(db);
    const key = `replay-${randomUUID().slice(0, 8)}`;
    const req = request(key);
    const principal = derivePrincipalId(req);

    const first = await store.initialize(req, principal, 1_700_000_000);
    expect(first.kind).toBe("INITIALIZED");

    // Same request replayed -> ALREADY_INITIALIZED with the SAME principal.
    const replay = await store.initialize(req, principal, 1_700_000_100);
    expect(replay).toEqual({
      kind: "ALREADY_INITIALIZED",
      principal_id: principal,
    });

    await db.close();
  });

  it("returns Conflict for a competing second first-owner request", async () => {
    const db = await freshDb(stack);
    const store = new OwnerBootstrapStore(db);
    const key = `conflict-${randomUUID().slice(0, 8)}`;
    const req = request(key);
    const principal = derivePrincipalId(req);

    const first = await store.initialize(req, principal, 1_700_000_000);
    expect(first.kind).toBe("INITIALIZED");

    // A different idempotency key with a different principal -> CONFLICT.
    const competing = request(`other-${randomUUID().slice(0, 8)}`);
    const second = await store.initialize(
      competing,
      derivePrincipalId(competing),
      1_700_000_100,
    );
    expect(second).toEqual({ kind: "CONFLICT" });

    // Durable state unchanged: still exactly one owner, the original.
    const owner = await store.readOwner();
    expect(owner!.owner_id).toBe(principal);

    await db.close();
  });

  it("survives concurrent competing first-owner attempts (real concurrency)", async () => {
    const db = await freshDb(stack);
    const store = new OwnerBootstrapStore(db);
    const keyA = `race-a-${randomUUID().slice(0, 8)}`;
    const keyB = `race-b-${randomUUID().slice(0, 8)}`;
    const reqA = request(keyA);
    const reqB = request(keyB);
    const principalA = derivePrincipalId(reqA);
    const principalB = derivePrincipalId(reqB);

    const [resA, resB] = await Promise.all([
      store.initialize(reqA, principalA, 1_700_000_000),
      store.initialize(reqB, principalB, 1_700_000_000),
    ]);

    // Exactly one canonical owner: one INITIALIZED, the other CONFLICT.
    const kinds = [resA.kind, resB.kind].sort();
    expect(kinds).toEqual(["CONFLICT", "INITIALIZED"]);
    const winner =
      resA.kind === "INITIALIZED"
        ? resA.principal_id
        : resB.kind === "INITIALIZED"
          ? resB.principal_id
          : (() => {
              throw new Error("expected exactly one INITIALIZED result");
            })();
    const owner = await store.readOwner();
    expect(owner!.owner_id).toBe(winner);

    await db.close();
  });

  it("maps unique violation to the canonical Conflict class", async () => {
    const db = await freshDb(stack);
    try {
      // Directly insert the same idempotency key twice -> the durable
      // boundary raises Conflict (SPEC-006), never Internal.
      await db.query(
        `INSERT INTO onboarding_owner
           (owner_id, idempotency_key, owner_email, state, correlation_id,
            created_at_unix_s, updated_at_unix_s)
         VALUES ($1, $2, $3, 'OWNER_PRINCIPAL_CREATED', $4, 1, 1)`,
        [randomUUID(), "dup-key", "dup@nexus.test", randomUUID()],
      );
      await expect(
        db.query(
          `INSERT INTO onboarding_owner
             (owner_id, idempotency_key, owner_email, state, correlation_id,
              created_at_unix_s, updated_at_unix_s)
           VALUES ($1, $2, $3, 'OWNER_PRINCIPAL_CREATED', $4, 1, 1)`,
          [randomUUID(), "dup-key", "dup@nexus.test", randomUUID()],
        ),
      ).rejects.toMatchObject({ code: ErrorCode.Conflict });
    } finally {
      await db.close();
    }
  });

  it("rejects bootstrap state whose principal does not match the owner row", async () => {
    const db = await freshDb(stack);
    const store = new OwnerBootstrapStore(db);
    const req = request(`mismatch-${randomUUID().slice(0, 8)}`);
    const principal = derivePrincipalId(req);
    await store.initialize(req, principal, 1_700_000_000);

    const otherPrincipal = randomUUID();
    const record = OwnerBootstrapStateRecord.parse({
      state: "OWNER_PRINCIPAL_CREATED",
      owner_email: req.owner_email,
      principal_id: otherPrincipal,
      correlation_id: randomUUID(),
      updated_at_unix_s: 1_700_000_100,
    });
    await expect(
      store.recordState(principal, record, req.correlation_id),
    ).rejects.toMatchObject({ code: ErrorCode.Validation });
    await db.close();
  });

  it("cannot write OWNER_AUTHORIZED without traversing the ladder (AUD-044)", async () => {
    const db = await freshDb(stack);
    const store = new OwnerBootstrapStore(db);
    const req = request(`jump-${randomUUID().slice(0, 8)}`);
    const principal = derivePrincipalId(req);
    await store.initialize(req, principal, 1_700_000_000);

    // Hostile: a caller supplies OWNER_AUTHORIZED while the durable row
    // sits at a LOWER rung (OWNER_IDENTITY_VERIFIED). The durable
    // boundary must reject the leap over OWNER_PRINCIPAL_CREATED.
    await db.query(
      `UPDATE onboarding_owner SET state = 'OWNER_IDENTITY_VERIFIED',
               updated_at_unix_s = 1_700_000_050 WHERE owner_id = $1`,
      [principal],
    );
    const record = OwnerBootstrapStateRecord.parse({
      state: "OWNER_AUTHORIZED",
      owner_email: req.owner_email,
      principal_id: principal,
      correlation_id: randomUUID(),
      updated_at_unix_s: 1_700_000_200,
    });
    await expect(
      store.recordState(principal, record, req.correlation_id),
    ).rejects.toMatchObject({ code: ErrorCode.Policy });

    // Exact-target readback: durable state never moved.
    const row = await store.readOwnerById(principal);
    expect(row!.state).toBe("OWNER_IDENTITY_VERIFIED");

    await db.close();
  });

  it("rejects a backwards ladder move at the durable boundary (AUD-044)", async () => {
    const db = await freshDb(stack);
    const store = new OwnerBootstrapStore(db);
    const req = request(`backward-${randomUUID().slice(0, 8)}`);
    const principal = derivePrincipalId(req);
    await store.initialize(req, principal, 1_700_000_000);

    // Hostile: OWNER_IDENTITY_VERIFIED is behind the current rung.
    const record = OwnerBootstrapStateRecord.parse({
      state: "OWNER_IDENTITY_VERIFIED",
      owner_email: req.owner_email,
      principal_id: principal,
      correlation_id: randomUUID(),
      updated_at_unix_s: 1_700_000_200,
    });
    await expect(
      store.recordState(principal, record, req.correlation_id),
    ).rejects.toMatchObject({ code: ErrorCode.Policy });

    const row = await store.readOwnerById(principal);
    expect(row!.state).toBe("OWNER_PRINCIPAL_CREATED");

    await db.close();
  });

  it("accepts the exact next ladder rung and idempotent re-assertion (AUD-044)", async () => {
    const db = await freshDb(stack);
    const store = new OwnerBootstrapStore(db);
    const req = request(`walk-${randomUUID().slice(0, 8)}`);
    const principal = derivePrincipalId(req);
    await store.initialize(req, principal, 1_700_000_000);

    // Seed a lower durable rung via the raw boundary is NOT possible
    // through the store (it enforces the ladder); walk from the initial
    // rung by inserting the first transition manually through SQL to
    // prove the store accepts only the exact next rung from ANY durable
    // state.
    await db.query(
      `UPDATE onboarding_owner SET state = 'OWNER_IDENTITY_VERIFIED',
               updated_at_unix_s = 1_700_000_050 WHERE owner_id = $1`,
      [principal],
    );

    // Exact next rung -> accepted.
    const next = OwnerBootstrapStateRecord.parse({
      state: "OWNER_PRINCIPAL_CREATED",
      owner_email: req.owner_email,
      principal_id: principal,
      correlation_id: randomUUID(),
      updated_at_unix_s: 1_700_000_100,
    });
    await store.recordState(principal, next, req.correlation_id);
    let row = await store.readOwnerById(principal);
    expect(row!.state).toBe("OWNER_PRINCIPAL_CREATED");

    // Idempotent re-assertion of the same rung -> accepted, no error.
    await store.recordState(principal, next, req.correlation_id);
    row = await store.readOwnerById(principal);
    expect(row!.state).toBe("OWNER_PRINCIPAL_CREATED");

    await db.close();
  });

  it("refuses state recording on a missing owner (AUD-044)", async () => {
    const db = await freshDb(stack);
    const store = new OwnerBootstrapStore(db);
    const ghostId = randomUUID();
    const record = OwnerBootstrapStateRecord.parse({
      state: "OWNER_PRINCIPAL_CREATED",
      owner_email: "ghost@nexus.test",
      principal_id: ghostId,
      correlation_id: randomUUID(),
      updated_at_unix_s: 1_700_000_100,
    });
    await expect(
      store.recordState(ghostId, record, randomUUID()),
    ).rejects.toMatchObject({ code: ErrorCode.NotFound });
    await db.close();
  });
});
