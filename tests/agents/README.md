# EP-017 Agent Failure and Abuse Tests

The EP-017 M4 failure and abuse suite lives in the owning crate:

- `crates/nexus-harness-adapters/tests/ep017_failure_registry.rs` - 10
  `ep017_failure_*` tests proving the capability-based registry fails
  safely: unknown capability never selects, unavailable /
  suspended / revoked agents excluded, deterministic tie-break is card
  id not vendor name, no vendor-name special case bypasses capability
  selection, duplicate registration is CONFLICT, missing unregister is
  NOT_FOUND, empty tenant / malformed capability request rejected, and
  no fabricated signal ever promotes an unmeasured card.
- `crates/nexus-harness-adapters/tests/ep017_failure_orchestrator.rs` -
  13 `ep017_failure_*` tests proving the parent orchestrator fails
  safely: zero budget fails before start, budget exhaustion fails the
  task and blocks new work, the agent cannot self-increase budget,
  cancel-before-start transitions without a delegation, cancel-while-
  running terminates the owned process, cancel transport failure fails
  closed (no orphan behind CANCELLED), duplicate cancel is idempotent,
  CANCELLED never becomes COMPLETED, revoked delegation cannot resume,
  completed delegation cannot reactivate, wrong artifact hash / missing
  name / cross-task artifact rejected, and partial side effect never
  fabricates SUCCEEDED (canonical ambiguous RUNNING state, no
  auto-retry of consequential work).
- `crates/nexus-harness-adapters/tests/ep017_failure_harness.rs` - 12
  `ep017_failure_*` tests proving the harness boundary fails safely:
  executable missing / non-zero exit / timeout / killed / malformed
  output all fail closed (typed UNAVAILABLE, never successful empty
  result), cancel terminates the owned process (no orphan), hostile
  injected text is data and cannot mint authority (no capability,
  tenant, budget, trust, or delegation mutation), terminal session
  rejects further messages, malformed review rejected, errors redact
  secrets, and authenticated tenant/principal is immutable across the
  full lifecycle.

Fixtures (e.g. `ScriptedRunner`) are CONTROLLED_TEST_FIXTURE: they
force crashes, malformed output, timeouts, and partial-side-effect
ambiguity deterministically. They do NOT certify a real external
coding-agent provider. Real provider certification for Codex / Claude
Code / Hermes / OpenClaw harnesses is deferred to LF-016 / the
integration owner recorded in the certification registry.

Run the suite with the M4 gate:

```sh
sh scripts/nodes/EP-017.sh M4
```
