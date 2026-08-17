# EP-023 operations (owned components)

Owned components: nexus-vision (contract crate: CameraEvent,
VisitorEvent, StreamRef, TwoWayAudioCapability, provider ports),
connectors/frigate (real Frigate 0.17.2 REST transport + adapter +
observability + frigate-diag), connectors/roku-home (real fail-closed
Roku provider ladder), tests/vision (cross-node E2E / LF-008).

## Health

- Build/unit health: `cargo check --workspace --locked` and
  `cargo test --locked -p nexus-vision ep023_unit`,
  `cargo test --locked -p nexus-frigate ep023_unit`,
  `cargo test --locked -p nexus-roku-home ep023_unit_roku`,
  `cargo test --locked -p nexus-vision-e2e ep023_e2e_`.
- Provider probe health: `cargo run -p nexus-frigate --bin frigate-diag
-- status` runs the REAL Frigate API probe against
  `FRIGATE_BASE_URL` and reports availability with redacted output.
  `-- recover` re-observes the provider after an outage; it never
  restarts infrastructure (observation-only).
- Live-stack health: the M3/M4 gates start the pinned mediamtx +
  Frigate containers, perform real RTSP/go2rtc/API probes, and tear
  down with zero orphans. LF-008 streams a REAL person photograph
  through the stack and asserts a real person detection event.

## Readiness

- A camera is Streaming only while go2rtc live evidence (format_name)
  is present; with the source dead it is NeverStreaming (real probe,
  no fabricated readiness).
- A stream reference is Unverified until real verification evidence
  exists (go2rtc normalization proof); verified() refuses an empty
  evidence reference.
- The Roku connector is ready to fail closed: inventory is empty and
  tier is UNAVAILABLE on this host (no hardware/credentials bound);
  it is never "ready to control" while no verified tier exists.
- Two-way audio is NOT certified on this node: certify() fails closed
  without a verified speaker path; the capability is never advertised
  from Frigate config metadata.

## Backup / restore

- No runtime database is owned by EP-023. Owned state is the
  append-only ledger (.agent/state/LEDGER.md), evidence under
  .agent/state/evidence/, and the declarative
  hardware/cameras/profiles.yaml conformance matrix. Backup = commit +
  tag (green/EP-023); restore = checkout the milestone commit or tag.
- Container images are pinned by digest (Frigate 0.17.2
  sha256:d4351369...7010, mediamtx v1.20.0
  sha256:25947caa...e336); re-pull restores the exact component.

## Upgrade

- Crate versions advance by ADR only (AGENTS.md dependency policy).
  The replacement boundary for the Frigate transport is any
  Frigate-compatible provider behind the CameraProvider/FrigateProvider
  ports; for the Roku ladder, any provider implementing the
  RokuHomeProvider port.
- Frigate image upgrades require a new pinned digest + gate re-run
  (M3/M4/LF-008 all verify the digest before starting).

## Disable

- Disable the Frigate connector by not running the container; every
  operation fails closed (Unavailable) and diag reports the provider
  down without restarting it.
- Disable the Roku connector by leaving RokuHomeProviderHost unbound;
  inventory stays empty and tier stays UNAVAILABLE.
- Disable live-fire proofs by not running the LF-008/M3/M4 gate
  scripts; the ambient workspace battery skips the live-stack tests
  via `#[ignore]`.
- Disable notification decisions by not consuming VisitorEvent; the
  decision function is deterministic and side-effect-free.

## Rollback

- Per-milestone commits (M1 e54c282, M2 3c78a4f, M3 7848fd6, M4
  bb63f15) and the node green tag green/EP-023 are the rollback
  points. Rollback to the previous milestone commit under LOOPS.md;
  never cross a completed green tag.
- The adapter rolls back every failed transport call to a classified
  error (Unavailable/Authorization/NotFound/External) with no partial
  side effect; telemetry never alters provider semantics.

## Certification boundary

- Roku HARDWARE_CERTIFICATION: DEFERRED (no physical device bound);
  the crate binds the provider port with an honest fail-closed
  implementation, never a fabricated capability.
- Physical camera classes: NOT_ASSERTED (hardware/cameras/profiles.yaml
  conformance DEFINED only).
- Two-way audio live certification: NOT_ASSERTED (requires a verified
  speaker/media path; LF-008 proves the honest fail-closed leg).
- WebRTC/RTSP media-level certification: NOT_ASSERTED without real
  media evidence; stream refs stay Unverified.
