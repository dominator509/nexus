# Self-Healing Runbook (EP-019 M4)

Operational diagnostics and bounded recovery for the self-healing
engineering loop (SPEC-018; ADR-026). Every remediation is traceable
through incident_id -> diagnosis_id -> patch_digest -> validation_result
-> security_result -> approval_id -> deployment_id -> verification_result
-> rollback_id -> final_state; telemetry carries redacted metadata only
(never secrets, credentials, private source context, or full model
prompts).

## State vocabulary

- Lifecycle: OBSERVE -> INCIDENT -> CORRELATE -> DIAGNOSE -> REPRODUCE
  -> PATCH_PROPOSED -> SANDBOX_VALIDATION -> SECURITY_VALIDATION ->
  APPROVAL -> STAGED_DEPLOYMENT -> POST_DEPLOY_VERIFICATION -> CLOSED.
- Terminals: REJECTED, UNREPRODUCIBLE, VALIDATION_FAILED,
  SECURITY_FAILED, ROLLED_BACK, BLOCKED.
- Diagnosis confidence: HYPOTHESIS (model explanation) -> SUPPORTED ->
  REPRODUCED -> VALIDATED (only reproducible evidence).

DETECTED != DIAGNOSED != REPRODUCED != PATCHED != VERIFIED != APPROVED
!= DEPLOYED != REMEDIATED. A model/agent can never declare its own fix
successful; only real observed verification closes an incident.

## Diagnostics

| Symptom | Diagnostic | Recovery |
| --- | --- | --- |
| Incident stuck in APPROVAL | `sh scripts/ledger.sh tail 30`; check approval digest vs patch digest | Re-issue approval bound to the exact patch digest; stale digest is POLICY rejection |
| Reproduction fails after patch | `sh scripts/ep019-m3-tests.sh` (real before/after) | Do NOT close; return to PATCH_PROPOSED and regenerate patch (bounded attempts) |
| Security gate fails | Run `sh scripts/security-check.sh`, `sh scripts/dependency-audit.sh`, `sh scripts/license-gate.sh`, `sh scripts/reality-gate.sh` | Reject patch (SECURITY_FAILED); a patch that weakens security is never deployed |
| Sandbox validation fails | `sh scripts/ep019-m3-tests.sh` on isolated copy | Return to PATCH_PROPOSED (bounded); unexpected scope expansion is VALIDATION_FAILED |
| Rollback needed | `sh scripts/ep019-m3-tests.sh` (rollback proof restores previous artifact) | Execute RollbackPlan bound to known previous artifact; health_verified must become true |
| Incident cannot be reproduced | Run reproduction twice on isolated copy | Record UNREPRODUCIBLE with evidence-limited status; never fabricate reproduction success |
| Model declares itself fixed | None — contract has no such state | Treat as HYPOTHESIS only; require real verification evidence |
| Duplicate incident records | Dedup key is tenant + error class + component (canonical) | Conflicts are idempotency failures; never merge across tenants |

## Bounded recovery commands

- `sh scripts/ep019-m1-tests.sh` — contract suite (vocabulary locking,
  lifecycle terminals, approval binding, dependency direction).
- `sh scripts/ep019-m2-tests.sh` — durable incident workflow contracts
  (vitest + tsc, vacuity guarded).
- `sh scripts/ep019-m3-tests.sh` — real integration chain (failing
  fixture -> reproduce -> patch -> verify -> rollback).
- `sh scripts/ep019-m4-tests.sh` — forced-failure suite.
- `cargo test --locked -p nexus-healing` — full nexus-healing suite.
- `cargo clippy --locked -p nexus-healing --all-targets -- -D warnings`.

## Retry bounds

Every loop is bounded: diagnosis attempts, patch-generation attempts,
validation attempts, deployment attempts, and rollback attempts each
have explicit maximums; repeated identical incidents are deduplicated by
canonical signature. There is no `while not fixed: ask model again`.

## Authority invariants

- The self-healing system can never expand its own authority: no policy
  self-modification, no capability self-grant, no approval lowering, no
  security gate disable, no budget increase, no tenant identity change.
- A successful remediation is a skill CANDIDATE only; it becomes a
  reusable skill exclusively through the EP-018 evaluation/signing/
  trust/install process. The self-healing system cannot directly install
  its own generated skills.

## Certification boundary

- Real OS-level sandbox isolation: DEFERRED (EP-040/EP-043).
- Real production canary deployment: DEFERRED to the deployment-owning
  node (EP-042/EP-043); the deterministic rollback state machine is
  proven by EP-019 now.
- Real Git/repository provider: owned by the node that owns the Git
  provider; reproduction/rollback are proven against isolated working
  copies.
