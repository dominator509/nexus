/**
 * EP-035 M3 integration: real IntegrationCard status truthfulness.
 *
 * Real status is evidence-based: CONFIGURED != AUTHENTICATED !=
 * REACHABLE != HEALTHY. The durable store rejects invalid ladder leaps
 * (credential-exists never implies HEALTHY) and capabilities are stored
 * as supplied data, never name-derived.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { randomUUID } from "node:crypto";
import { IntegrationCardRequest, ErrorCode } from "@nexus/setup";
import { IntegrationStateStore } from "../../stores/integration-state.store.js";
import { freshDb, startStack, stopStack, type TestStack } from "./harness.js";

describe("ep035_integration_integration_state", () => {
  let stack: TestStack;

  beforeAll(async () => {
    stack = await startStack();
  }, 180000);

  afterAll(async () => {
    await stopStack(stack);
  }, 60000);

  function cardRequest(provider = "home-assistant"): IntegrationCardRequest {
    return IntegrationCardRequest.parse({
      integration_id: randomUUID(),
      provider_name: provider,
      correlation_id: randomUUID(),
    });
  }

  it("persists the full CONFIGURED->AUTHENTICATED->REACHABLE->HEALTHY ladder with evidence", async () => {
    const db = await freshDb(stack);
    const store = new IntegrationStateStore(db);
    const req = cardRequest();
    const now = 1_700_000_000;

    const created = await store.create(
      req.integration_id,
      req.provider_name,
      now,
    );
    expect(created.status).toBe("UNCONFIGURED");

    const configured = await store.recordStatus(
      req.integration_id,
      req,
      "CONFIGURED",
      now + 1,
    );
    expect(configured.status).toBe("CONFIGURED");
    expect(configured.configured_at_unix_s).toBe(now + 1);
    // Credential-exists never implies health: HEALTHY is not yet set.
    expect(configured.healthy_at_unix_s).toBeNull();

    const authenticated = await store.recordStatus(
      req.integration_id,
      req,
      "AUTHENTICATED",
      now + 2,
    );
    expect(authenticated.status).toBe("AUTHENTICATED");

    const reachable = await store.recordStatus(
      req.integration_id,
      req,
      "REACHABLE",
      now + 3,
    );
    expect(reachable.status).toBe("REACHABLE");
    expect(reachable.reachable_at_unix_s).toBe(now + 3);

    const healthy = await store.recordStatus(
      req.integration_id,
      req,
      "HEALTHY",
      now + 4,
    );
    expect(healthy.status).toBe("HEALTHY");
    expect(healthy.healthy_at_unix_s).toBe(now + 4);

    // Exact-target readback: durable row matches.
    const row = await store.read(req.integration_id);
    expect(row!.status).toBe("HEALTHY");
    expect(row!.provider_name).toBe("home-assistant");

    await db.close();
  });

  it("rejects invalid ladder leaps (UNCONFIGURED->HEALTHY is Policy)", async () => {
    const db = await freshDb(stack);
    const store = new IntegrationStateStore(db);
    const req = cardRequest();
    const now = 1_700_000_000;

    await store.create(req.integration_id, req.provider_name, now);
    await expect(
      store.recordStatus(req.integration_id, req, "HEALTHY", now + 1),
    ).rejects.toMatchObject({ code: ErrorCode.Policy });

    const row = await store.read(req.integration_id);
    expect(row!.status).toBe("UNCONFIGURED"); // unchanged

    await db.close();
  });

  it("stores capabilities as supplied data, never name-derived", async () => {
    const db = await freshDb(stack);
    const store = new IntegrationStateStore(db);
    const req = cardRequest("home-assistant");
    const now = 1_700_000_000;

    await store.create(req.integration_id, req.provider_name, now);
    // The provider name alone advertises nothing; capability data must
    // be explicitly supplied.
    const row0 = await store.read(req.integration_id);
    expect(row0!.capability_json).toEqual([]);

    const row1 = await store.setCapabilities(req.integration_id, [
      { capability: "light.switch", evidence: "capability-endpoint" },
    ]);
    expect(Array.isArray(row1.capability_json)).toBe(true);

    const row2 = await store.read(req.integration_id);
    expect(row2!.capability_json).toEqual([
      { capability: "light.switch", evidence: "capability-endpoint" },
    ]);

    await db.close();
  });

  it("returns NotFound for status transitions on a missing card", async () => {
    const db = await freshDb(stack);
    const store = new IntegrationStateStore(db);
    const req = cardRequest();

    await expect(
      store.recordStatus(req.integration_id, req, "CONFIGURED", 1_700_000_000),
    ).rejects.toMatchObject({ code: ErrorCode.NotFound });

    await db.close();
  });
});
