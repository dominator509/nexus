# SECURITY

## Goals

Protect people, homes, businesses, communications, credentials, agents, devices, and infrastructure while preserving low-latency local control. Security is enforced by identity, policy, capability boundaries, network segmentation, data minimization, verification, and audit. Model alignment is never a security boundary.

## Threat summary

Threats include stolen tokens, malicious household guests, cloned voices, compromised phones, hostile web and email content, prompt injection, rogue connectors and skills, vulnerable sidecars, compromised IoT devices, lateral movement, malicious providers, supply-chain tampering, cross-tenant access, insider misuse, data exfiltration, replay, duplicated external actions, unsafe agent code, and update compromise.

## Authentication

- Keycloak OIDC and OAuth2 with passkeys first.
- Short-lived access tokens, refresh rotation, audience and issuer validation, token revocation, device binding where supported, and service-account separation.
- mTLS device identity for control-plane nodes, edge, connectors, and high-trust workers.
- Step-up uses platform biometric plus passkey or equivalent cryptographic assertion.
- Voice, face, BLE, occupancy, geofence, and behavior contribute confidence but cannot authorize R3 or R4 actions.

## Authorization

- OpenFGA stores relationships among people, households, businesses, devices, resources, agents, and roles.
- OPA evaluates current context, risk, time, presence, device trust, data class, network, provider certification, and approval.
- Action Gateway returns allow, request approval, suggest, or deny and records a policy receipt.
- Default deny applies at every external boundary.
- Connectors receive scoped short-lived capability tokens, never owner tokens.

## Trust boundaries

Public ingress -> Caddy -> authenticated control API. Control API -> application ports. Applications -> policy and repositories. Connectors and agents -> gateway -> policy -> scoped tools. Sidecars have isolated networks, filesystems, users, secrets, and egress. Home IoT, cameras, guests, trusted clients, servers, and management interfaces use separate network zones.

## Input safety

Every input is untrusted: HTTP, MCP, A2A, event, webhook, email, attachment, social content, browser page, OCR, transcript, tool result, model output, agent artifact, connector response, file, device state, and operator import. Inputs have size, type, schema, recursion, rate, timeout, decompression, archive, MIME, and content validation. Documents are scanned before agent or model access.

## Prompt injection and agents

Untrusted content is labeled and separated from system instructions. Models receive only capability descriptions, not raw credentials. Tool output cannot introduce new tools or scopes. Agent execution uses isolated worktrees, containers, restricted network, resource quotas, immutable base images, and artifact review. Shell capability is never universal and excludes host secret stores and production networks.

## Secrets

OpenBao is central in production. SOPS and age encrypt bootstrap and disaster-recovery configuration. Mobile uses Secure Enclave or Keychain and Android Keystore. Desktop uses OS keychain. Secret values are never logged, embedded in events, memories, prompts, support bundles, or UI exports. Secret references are resolved at the last responsible service and are zeroed where language support permits.

## Network

WireGuard and Headscale-compatible private mesh provide node reachability. mTLS authenticates services. Caddy uses modern TLS and security headers. Management ports, databases, NATS, Temporal, OpenBao, Home Assistant, Frigate, Asterisk, and observability are not public by default. Egress is allowlisted per component. Cameras and IoT cannot initiate connections to trusted workstations.

## Web and API

Strict CORS allowlist, Origin validation for MCP, CSRF protection for browser sessions, secure HttpOnly SameSite cookies, CSP, HSTS in production, request body limits, upload quarantine, rate limits by principal and IP, replay protection, idempotency, safe redirects, and structured errors. Tenant is derived from authenticated bindings, never a caller-controlled header.

## Mobile and desktop

Device registration, attestation where available, secure storage, certificate pinning policy that supports safe rotation, jailbreak or root risk evidence without blanket false security, push messages with opaque references, and remote revocation. Sensitive screens obscure app-switcher snapshots where supported.

## Data protection

Classification, purpose, retention, export, deletion, private response routing, and API egress follow SPEC-020. Backups are encrypted before leaving the trust zone. Camera, voice, call, fax, email, and business content have separate retention controls.

## Supply chain

Pinned dependencies and images, SBOM, signatures, attestations, vulnerability and license scans, immutable GitHub Action SHAs, branch protection, review, reproducible build evidence, and staged updates. Copyleft sidecars are isolated and notices are preserved. Noncommercial models and datasets are prohibited.

## Migration safety

Use expand, migrate, verify, contract. Additive migrations first. Destructive changes require backup, restore rehearsal, compatibility window, explicit human approval, and rollback or forward-fix plan. Production data is never copied into development.

## Abuse prevention

Rate limits, cost limits, message and call quotas, social publish approval, connector circuit breakers, anomaly detection, brute-force protection, anti-replay, and administrative approval. Bulk external communication and money or purchase capabilities are disabled until separately specified and certified.

## Security checks

`scripts/security-check.sh` runs secret scanning (tracked .env and secret-pattern scan over tracked files), dependency vulnerability scans (cargo audit, pnpm audit, python security audit), static analysis (cargo clippy -D warnings), policy/authorization tests (the real nexus-security-core failure battery: secrets, deny-by-default policy, authz, redaction, container termination), and the license and reality gates. A critical exploitable finding blocks release. IaC/container image scans, image signature verification, forbidden-route scans, and tenant-isolation integration tests are NOT part of this script's asserted surface; those capabilities are certified by their owning gates when they exist (see release trust chain, RX-009/RX-010).

## Security stop conditions

Stop when an action could destroy production or user data, produce an unspecified irreversible effect, require a legal or security judgment not answered by the specs, expose a secret, weaken identity or policy, or exhaust the Graphlock ladder. Do not work around the condition.
