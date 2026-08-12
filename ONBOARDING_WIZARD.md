# ONBOARDING WIZARD

## Product requirement

Nexus Setup is a core application. Ordinary users must never manually install PostgreSQL, NATS, Temporal, Keycloak, OpenFGA, OPA, OpenBao, Caddy, Home Assistant, Frigate, model gateways, or monitoring components.

## Flow

1. Welcome, privacy model, deployment ownership, and recovery warning.
2. Create owner with passkey and printable recovery kit.
3. Choose managed cloud, BYOC, existing SSH server, fully local, or advanced hybrid.
4. Authorize cloud provider locally and choose an understandable size profile.
5. Hardware profiler benchmarks CPU, RAM, GPU or NPU, storage, network, audio, Bluetooth, and virtualization.
6. Deployment planner shows placement, recurring cost, privacy, fallbacks, and disabled features before apply.
7. Provision infrastructure through OpenTofu and cloud-init, install signed containers, and establish the private mesh.
8. Enroll the home edge with a one-time QR code that becomes mTLS device identity.
9. Discover devices and services, assign friendly names, rooms, businesses, and owners.
10. Add household members with adult, child, guest, contractor, or custom relationships.
11. Connect Google, Microsoft, GitHub, Hydra, social, telephony, storage, and optional AI providers through OAuth or scoped credentials.
12. Enroll voice samples, choose wake word, choose local or API speech providers, test each room, and verify hardware mute.
13. Run Sentinel baseline and propose segmentation or DNS changes.
14. Configure encrypted backups and run an immediate restore verification to a scratch target.
15. Complete a guided live test of chat, voice, home control, approval, and notification.

## Failure handling

Every step is resumable and idempotent. Credentials stay in the local setup process until transferred directly to OpenBao or the target provider. A failed component does not force a complete restart. The wizard shows plain-language cause, exact evidence, safe retry, rollback, and support bundle creation.
