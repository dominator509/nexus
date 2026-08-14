# LF-017 Durable Human Approval (EP-006 M5)

Generated: 2026-08-14T15:56:55Z
Node: EP-006
Command: sh scripts/live-fire/LF-017.sh

## Real proof (no mocks, no in-memory engine)
- ep006_integration_worker_restart_delayed_approval_exactly_once: REAL
  Temporal server 1.31.2 (digest b5ecdb82...) + REAL postgres:18.4
  (digest a02db8ca...); workflow started on worker A, worker A shut down
  while step 2 awaited approval, the delayed approval signal was delivered
  while NO worker polled (the mobile 'approve later' path), worker B
  resumed from recorded history; each effect executed exactly once per
  idempotency key (counting test activity, TESTING.md test zone).
- ep006_integration_replay_recorded_history_succeeds: the completed
  workflow's recorded history replays through the worker bundle with no
  DeterminismViolationError (replay compatibility invariant).

## Observed sentinel
LF-017: ok

## Post-proof hygiene
- EP-006 orphan audit: ok (zero nexus-ep006 containers/networks/volumes/
  temporal-server processes after the live-fire run).
