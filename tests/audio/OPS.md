# EP-022 operations (owned components)

Owned components: nexus-audio (contract crate), nexus-assist-satellite
(adapter core), connectors/wyoming (real Wyoming protocol transport),
nexus-bluetooth-audio (real D-Bus/BlueZ connector), tests/audio
(cross-node E2E / LF-026).

## Health

- Build/unit health: `cargo check --workspace --locked` and
  `cargo test --locked -p nexus-audio ep022_unit`,
  `cargo test --locked -p nexus-assist-satellite ep022_unit`,
  `cargo test --locked -p nexus-bluetooth-audio ep022_failure`,
  `cargo test --locked -p nexus-audio-e2e ep022_e2e`.
- Connector probe health: `cargo run -p nexus-bluetooth-audio --bin
bluetooth-diag -- status` runs the REAL system-bus probe and reports
  `"bus_ok"` and `"bluez"` observations.
- Wyoming transport health: the M3 gate starts the real container,
  performs a protocol-level describe/info handshake, and tears the
  container down with zero orphans.

## Readiness

- A satellite is ready (LISTENING) only when its local wake gate is
  bound; an unbound gate fails closed UNAVAILABLE (SPEC-012 behavior
  3). Hardware mute makes the satellite not ready (behavior 9).
- The Bluetooth connector is ready to fail closed when the system bus
  is reachable; it is never "ready to connect" while org.bluez has no
  owner (real probe, no fabricated readiness).
- The Wyoming container is ready only after the real protocol
  handshake succeeds; a bare TCP connect is not readiness.

## Backup / restore

- No runtime database is owned by EP-022. Owned state is the
  append-only ledger (.agent/state/LEDGER.md), evidence under
  .agent/state/evidence/, and the declarative
  hardware/voice/profiles.yaml conformance matrix. Backup = commit +
  tag (green/EP-022); restore = checkout the milestone commit or tag.
- Container images are pinned by digest (wyoming-openwakeword
  sha256:52cb1168...d42b); re-pull restores the exact component.

## Upgrade

- Crate versions advance by ADR only (AGENTS.md dependency policy).
  The replacement boundary for the minimal D-Bus client is the
  zbus/bluer crates; for the Wyoming server, any Wyoming-protocol
  compatible server behind the WyomingProvider port.

## Disable

- Disable the Bluetooth connector by leaving the connect policy at
  default-deny (DenyByDefaultPolicy) and/or not running the connector;
  every operation fails closed.
- Disable the Wyoming transport by not running the container; the M3
  gate is the lifecycle owner and never leaves orphans.
- Disable a satellite by stop_listening(); hardware mute is
  authoritative and never auto-resumes.

## Rollback

- Per-milestone commits (M1 7810b9f, M2 478e702, M3 566e422, M4
  7f2866e) and the node green tag green/EP-022 are the rollback
  points. Rollback to the previous milestone commit under LOOPS.md;
  never cross a completed green tag.
- The connector rolls back every failed connect to DISCONNECTED (no
  partial side effect) and policy denials never create state.

## Certification boundary

- Bluetooth/A2DP transport: DEFERRED to hardware ownership
  (EP-040/EP-043); never claimed from this node.
- Physical hardware classes: NOT_ASSERTED (hardware/voice/profiles.yaml
  conformance DEFINED only).
- Production wake-model certification: DEFERRED per SPEC-019.
