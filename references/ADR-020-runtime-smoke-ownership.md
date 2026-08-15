# ADR-020 - Runtime Smoke Ownership

Status: Accepted
Date: 2026-08-14
Owner: hermes-nexus-main
Node: EP-044

## Context

The global runtime smoke stage in `scripts/smoke-test.sh` was activated at
`at-least EP-012`. EP-012 is DONE, so the stage ran for the first time and
failed with `NEXUS_BASE_DOMAIN: parameter not set` (and, with the canonical
domain, `curl: (6) Could not resolve host: nexus.test`). The bounded
verification ladder proved this is a GRAPH / BLUEPRINT GATE-OWNERSHIP
DEFECT: no node in the 44-node graph owned or created the Nexus runtime
(`apps/control-plane`, `apps/edge`, `apps/cli`, `infra/compose/` appear in
ARCHITECTURE.md and COMPONENT_REGISTRY but in no node fence/ExecPlan/
contract; the apps/README "control-plane node (graph EP-007+)" is a phantom;
zero `main.rs` exists). EP-013 was recorded NODE_BLOCKED with reason
`GLOBAL_GATE_PREREQUISITE_UNOWNED` (evidence:
`.agent/state/evidence/ep013-block/EP-013-NODE_BLOCKED.md`).

The owner GraphLock amendment of 2026-08-14 resolved the defect by creating a
dedicated runtime node: EP-044, inserted between EP-013 and EP-014
(EP-014's DEPS rewired EP-013 -> EP-044).

## Decision

1. EP-044 owns the Nexus Control Plane Runtime and the runtime smoke.
2. `scripts/smoke-test.sh` activates the runtime smoke only at
   `at-least EP-044`.
3. Before EP-044 is DONE, the stage prints
   `runtime smoke: not-applicable-before EP-044` - an explicit
   not-applicable classification, NOT a PASS claim for runtime
   functionality. The overall `smoke test: ok` stays green because the
   requirement is not yet applicable, not because the smoke passed.
4. At/after EP-044 is DONE, the runtime smoke MUST run and MUST fail if the
   runtime is absent or unhealthy (fail closed).
5. A gate-ownership regression test (`tests/runtime/smoke-gate-regression.sh`)
   proves: stage < runtime owner -> runtime smoke not invoked; activation
   wiring points at EP-044; stage >= owner + runtime absent -> smoke fails.
6. The runtime smoke assertions themselves are unchanged
   (`/healthz` `{"status":"healthy"}`, `/readyz` `{"ready":true}`,
   `/v1/capabilities` non-empty); the amendment changes WHEN the gate becomes
   mandatory, not WHAT it proves.

## Invariant

No gate may become mandatory before the node(s) that create its real
dependency are DONE.

## Consequences

EP-013's node-verify becomes satisfiable (its smoke stage is
not-applicable-before EP-044). EP-044 must implement a real runnable
control-plane server; until then every node-verify reports the runtime smoke
as not-applicable, never as a false pass.

## Compatibility

Additive. The runtime smoke assertions are not weakened. Prior green tags and
ledger history preserved.
