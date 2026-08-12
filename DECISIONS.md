# DECISIONS

| ID | Decision | Status |
| --- | --- | --- |
| ADR-0001 | Rust-first polyglot monorepo | Accepted |
| ADR-0002 | PostgreSQL and pgvector first with graph repository escape hatch | Accepted |
| ADR-0003 | NATS JetStream plus transactional outbox | Accepted |
| ADR-0004 | Temporal for durable workflows | Accepted |
| ADR-0005 | DeepSeek V4 Flash V1 ReflexProvider and Microbrain drop-in contract | Accepted |
| ADR-0006 | Bifrost preferred behind replaceable ModelGateway | Accepted |
| ADR-0007 | Nexus Model Router Contract with adaptable RouteLLM-compatible policy | Accepted |
| ADR-0008 | Keycloak, OpenFGA, OPA, and deterministic Action Gateway | Accepted |
| ADR-0009 | OpenBao, SOPS age, device stores, WireGuard, Headscale, and mTLS | Accepted |
| ADR-0010 | Home Assistant and Frigate as primary home and vision sidecars | Accepted |
| ADR-0011 | Local voice defaults with provider fallbacks and custom wake weights | Accepted |
| ADR-0012 | Asterisk, ICTFax, universal email, and communications router | Accepted |
| ADR-0013 | OPNsense, OpenWrt, AdGuard Home, and tiered Sentinel | Accepted |
| ADR-0014 | Postiz isolated sidecar and direct social APIs | Accepted |
| ADR-0015 | Hydra is the CRM bounded context beneath Nexus | Accepted |
| ADR-0016 | Universal Connector Contract and multi-language SDKs | Accepted |
| ADR-0017 | One Tauri setup wizard and common deployment manifests | Accepted |
| ADR-0018 | Flutter mobile plus React PWA and Tauri desktop | Accepted |
| ADR-0019 | License classes and copyleft process isolation | Accepted |
| ADR-0020 | Stage-aware live-fire during graph and all-proofs final ship gate | Accepted |
| ADR-0021 | Self-hosted local filesystem or NAS default; SeaweedFS scalable; MinIO compatibility only | Accepted |

## ADR details

Each accepted decision is binding until replaced by a later ADR. The full rationale, alternatives, consequences, license boundary, rollback, and evidence are recorded when the owning node implements it. EP-000 verifies all versions and may update patch versions with an ADR if a security advisory requires it.

## Adding a decision

Copy `.agent/templates/adr-template.md`, assign the next number, cite the triggering spec and repository evidence, list at least one rejected alternative, record compatibility and rollback, update the table, and append a ledger event before implementation relies on the decision.
