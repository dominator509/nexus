# Self-Healing Integration (EP-019 M3)

Real integration suite for the self-healing engineering loop (SPEC-018;
ADR-026). This directory exercises the REAL process boundary against a
REAL controlled failing fixture and a REAL patch artifact:

- `fixtures/failing-worker.sh` — CONTROLLED_TEST_FIXTURE (TESTING.md):
  a real deterministic executable with a real logic bug. It checks a
  hard-coded wrong filename, so it crashes (exit 1) even when the
  operator provides the correct marker path. This is the "controlled
  failing software" the EP-019 owner directive permits for generating
  incidents.
- `fixtures/worker-fix.patch` — the REAL patch artifact. Applying it to
  an isolated working copy fixes the filename check; the same
  reproduction that FAILED before the patch PASSES after it.
- `test_ep019_integration_healing_loop.py` — the integration suite.

The strongest real chain proven here (directive section 23):

```
real failing fixture/process
  -> actual incident (real subprocess crash = incident signal)
  -> real diagnosis/orchestration (canonical incident + digest)
  -> real patch artifact (real SHA-256 digest)
  -> patch applied to an isolated working copy (real `patch` tool)
  -> failing reproduction reproduced BEFORE the patch
  -> same reproduction passes AFTER the patch
  -> regression / fail-closed boundary preserved
  -> approval boundary (approval binds to the exact patch digest)
  -> staged/internal deployment (isolated working copy)
  -> verification (observed exit status)
  -> closure / rollback proof (restore previous artifact)
```

No mocks, no in-memory production engine substitute: the fixture is a
real executable, the patch is a real diff, and every reproduction is a
real subprocess with an observed exit status. Production behavior never
generates fake incidents.

## Certification boundary

- Real OS-level sandbox isolation certification: DEFERRED (EP-040/EP-043).
- Real production canary deployment certification: DEFERRED to the node
  that owns deployment (EP-042/EP-043).
- Real Git/repository provider certification: owned by the node that owns
  the Git provider; this suite proves the deterministic before/after
  reproduction and rollback state machine now.

## Gate

`sh scripts/ep019-m3-tests.sh` runs the suite through the real pytest
machinery with a vacuity guard (a zero-match selection fails the gate).
