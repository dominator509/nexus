/**
 * EP-035 M3 integration: real one-time enrollment tokens.
 *
 * Proves against REAL PostgreSQL 18.4 that a fresh token is accepted
 * exactly once, replay is denied, expired is denied, revoked is denied,
 * and the credential secret is never persisted in plaintext.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { randomUUID } from "node:crypto";
import { EnrollmentCredential } from "@nexus/setup";
import {
  EnrollmentTokenStore,
  hashSecret,
} from "../../stores/enrollment-token.store.js";
import { freshDb, startStack, stopStack, type TestStack } from "./harness.js";

describe("ep035_integration_enrollment_token", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  function makeCredential(
    nowUnixS: number,
    ttlS = 3600,
  ): {
    credential: EnrollmentCredential;
    secret: string;
  } {
    const secret = `bootstrap_secret_${randomUUID().replaceAll("-", "")}`;
    const credential = EnrollmentCredential.parse({
      credential_id: randomUUID(),
      kind: "BOOTSTRAP_TOKEN",
      issued_at_unix_s: nowUnixS,
      expires_at_unix_s: nowUnixS + ttlS,
      state: "ISSUED",
      nonce: `nonce_${randomUUID().replaceAll("-", "")}`,
      secret,
    });
    return { credential, secret };
  }

  it("accepts a fresh token exactly once and denies replay", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);
    const now = 1_700_000_000;
    const { credential, secret } = makeCredential(now);

    const issued = await store.issue(credential);
    expect(issued.credential_id).toBe(credential.credential_id);
    // The redacted shape never exposes secret material.
    expect("secret" in issued).toBe(false);
    expect("nonce" in issued).toBe(false);

    // Fresh token + correct secret -> claim succeeds.
    expect(await store.claim(credential.credential_id, secret, now + 1)).toBe(
      true,
    );

    // Replay -> denied (durable one-time boundary).
    expect(await store.claim(credential.credential_id, secret, now + 2)).toBe(
      false,
    );

    // Exact-target readback: state is USED.
    const row = await store.read(credential.credential_id);
    expect(row!.state).toBe("USED");
    expect(row!.used_at_unix_s).toBe(now + 1);

    await db.close();
  });

  it("denies consumption by credential ID alone (AUD-043)", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);
    const now = 1_700_000_000;
    const { credential, secret } = makeCredential(now);

    await store.issue(credential);

    // Wrong secret -> claim denied; the credential is NOT consumed.
    expect(
      await store.claim(credential.credential_id, "wrong-secret", now + 1),
    ).toBe(false);
    let row = await store.read(credential.credential_id);
    expect(row!.state).toBe("ISSUED");

    // Empty secret -> claim denied (hostile caller knowing only the ID).
    expect(await store.claim(credential.credential_id, "", now + 1)).toBe(
      false,
    );
    row = await store.read(credential.credential_id);
    expect(row!.state).toBe("ISSUED");

    // The correct secret still works afterwards (nothing was consumed).
    expect(await store.claim(credential.credential_id, secret, now + 1)).toBe(
      true,
    );
    row = await store.read(credential.credential_id);
    expect(row!.state).toBe("USED");

    await db.close();
  });

  it("denies an expired token", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);
    const now = 1_700_000_000;
    const { credential, secret } = makeCredential(now, 100);

    await store.issue(credential);
    // Claim after expiry -> denied.
    expect(await store.claim(credential.credential_id, secret, now + 200)).toBe(
      false,
    );
    const row = await store.read(credential.credential_id);
    expect(row!.state).toBe("ISSUED"); // never flipped to USED

    await db.close();
  });

  it("denies a revoked token and revoke is permanent", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);
    const now = 1_700_000_000;
    const { credential, secret } = makeCredential(now);

    await store.issue(credential);
    expect(await store.revoke(credential.credential_id, now + 1)).toBe(true);
    // Revoked token can never be claimed.
    expect(await store.claim(credential.credential_id, secret, now + 2)).toBe(
      false,
    );
    // Second revoke returns false (already revoked).
    expect(await store.revoke(credential.credential_id, now + 3)).toBe(false);
    const row = await store.read(credential.credential_id);
    expect(row!.state).toBe("REVOKED");

    await db.close();
  });

  it("never persists the raw secret; verification uses the hash", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);
    const now = 1_700_000_000;
    const { credential, secret } = makeCredential(now);

    await store.issue(credential);
    const row = await store.read(credential.credential_id);
    expect(row!.secret_hash).toBe(hashSecret(secret));
    expect(row!.secret_hash).not.toContain(secret);

    // Verify the presented secret against the stored hash.
    expect(await store.verifySecret(credential.credential_id, secret)).toBe(
      true,
    );
    expect(
      await store.verifySecret(credential.credential_id, "wrong-secret"),
    ).toBe(false);

    await db.close();
  });

  it("races two concurrent claims; exactly one wins", async () => {
    const db = await freshDb(stack);
    const store = new EnrollmentTokenStore(db);
    const now = 1_700_000_000;
    const { credential, secret } = makeCredential(now);

    await store.issue(credential);
    const [a, b] = await Promise.all([
      store.claim(credential.credential_id, secret, now + 1),
      store.claim(credential.credential_id, secret, now + 1),
    ]);
    expect([a, b].filter(Boolean).length).toBe(1);
    const row = await store.read(credential.credential_id);
    expect(row!.state).toBe("USED");

    await db.close();
  });
});
