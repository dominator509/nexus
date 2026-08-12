# DEPLOYMENT

## Environments

- Development: local workstation and container dependencies.
- Test: ephemeral isolated tenants and real local dependencies.
- Staging: production-shaped user-owned VPS and home-edge lab with test accounts and hardware.
- Production: user-owned or managed profile, signed stable release, real owner and providers.

## Reference topology

Caddy terminates public TLS for the web, callback, and authenticated API surface. The control plane, PostgreSQL, NATS, Temporal, Keycloak, OpenFGA, OPA, OpenBao, and GlitchTip run on private container networks. Headscale or compatible control coordinates WireGuard peers. Home edge connects outbound over the mesh and hosts Home Assistant, Frigate, voice, Bluetooth, and device adapters. Sidecars receive only required networks and secrets.

## Build artifacts

- Signed multi-architecture OCI images by component.
- Tauri installers for Windows, macOS, and Linux.
- Flutter iOS and Android build artifacts.
- `nexus-offline-bundle-<version>.tar.zst` with images, models, manifests, SBOMs, signatures, migrations, licenses, and recovery tools.
- OpenTofu modules and cloud-init payloads.
- Signed release manifest with compatibility and rollback metadata.

## Release flow

Green graph -> release candidate tag -> all CI -> staging deployment -> all active live-fire -> backup and restore -> update and rollback drill -> security and license review -> signed stable release -> exact manual production promotion command.

## Provisioning

Nexus Setup invokes provider adapters locally. OpenTofu creates compute, network, firewall, volumes, DNS, object storage where selected, and SSH keys. Cloud-init hardens the OS, installs the pinned runtime, enrolls the node, pulls signed artifacts, and returns a one-time status. Provider master credentials are discarded unless ongoing infrastructure management is enabled.

## Migrations

Run database and event compatibility prechecks, take an encrypted backup, apply additive migrations, start new readers and writers, verify state and outbox, observe, then contract old fields only in a later compatible release. A destructive migration is a manual stop.

## Deployment command

Production deployment is MANUAL. EP-043 prints a command shaped as:

`NEXUS_RELEASE=<signed-version> NEXUS_TARGET=<deployment-id> sh scripts/deploy-release.sh`

The actual version and target are emitted only after the ship gate. This blueprint does not authorize executing that command.

## Post-deploy verification

Health and readiness, identity login, capability discovery, event lag, workflow worker, DeepSeek reflex, home edge, backup status, Sentinel status, provider certification status, mobile push, and release telemetry must pass. A failure triggers the documented rollback before accepting traffic.

## Stop conditions

Missing signatures, failed backup, incompatible migration, failed required live-fire, unknown license, critical advisory, policy regression, inability to roll back, or absent human production authorization.
