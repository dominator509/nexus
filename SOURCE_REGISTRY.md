# SOURCE REGISTRY

## Purpose

EP-000 verifies every source, release, license, artifact digest, and security advisory before implementation. This registry records the authoritative upstream projects selected during blueprint generation. A version may advance only through an ADR and a green dependency, license, and compatibility gate.

## Primary standards

- MCP specification: https://modelcontextprotocol.io/specification/2025-11-25
- A2A specification and TCK: https://github.com/a2aproject/A2A at v1.0.1
- Agent Skills specification: https://agentskills.io/specification, snapshot dated 2026-08-12
- OpenTelemetry: https://opentelemetry.io and Collector release 0.158.0
- OAuth 2.0, OpenID Connect, WebAuthn, W3C Trace Context, CloudEvents, JSON Schema 2020-12, OpenAPI 3.1, AsyncAPI 3, and S3 API references are pinned by EP-000 into `references/standards/`.

## Selected upstreams

The exact release set and license posture are in VERSIONS.lock.yaml and COMPONENT_REGISTRY.yaml. High-risk notes:

- Bifrost is preferred but not architectural; the `ModelGateway` contract supports replacement.
- Postiz, Asterisk, ICTFax, AdGuard Home, Suricata, OpenWrt, Wazuh, and MinIO carry copyleft obligations and remain separate processes or appliances.
- MinIO Community is archived and remains compatibility-only. SeaweedFS is the preferred scalable self-hosted S3-compatible option; local filesystem or NAS is the simplest default.
- openWakeWord runtime code is acceptable, but bundled upstream model weights with noncommercial restrictions must not ship. Nexus trains and signs its own wake-word weights from consented or commercially compatible data.
- Roku Home local capabilities are unknown until device-specific verification. The provider advertises only observed and tested capabilities.

## Verification record

EP-000 writes `references/SOURCE_VERIFICATION.json` containing URL, tag, commit, artifact digest, license digest, release date, advisory status, and verification timestamp for each lock.
