# SPEC-013 - Sentinel, Firewall, DNS, Network Detection, and Endpoint Security

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define lightweight core protection, optional advanced profiles, evidence correlation, containment, and human-governed remediation.

## Canonical terms

Sentinel, DeviceFingerprint, Baseline, SecurityEvent, Incident, Quarantine, OPNsense, OpenWrt, AdGuard, Suricata, Zeek, CrowdSec, EndpointSensor. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Sentinel Core uses firewall telemetry, AdGuard DNS, inventory, expected destinations, flow baselines, identity events, and Nexus system events.
2. OPNsense is primary serious firewall; OpenWrt is supported for embedded and consumer installations.
3. Enhanced profile adds Suricata; Advanced adds Zeek; Endpoint adds Wazuh or osquery; CrowdSec is optional reputation enforcement.
4. Every device has expected protocols, destinations, internal access, baseline, owner, firmware, provider, and trust class.
5. Automated containment is limited to preauthorized high-confidence reversible rules and always notifies the owner.
6. Destructive remediation, credential rotation, wipes, factory resets, or broad lockouts require human procedure.
7. Honeypots and honeytokens are optional high-signal sensors isolated from real data.
8. No default HTTPS interception is permitted. Metadata and endpoint telemetry are preferred.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Heavy enterprise SOC on every home
- LLM as sole detector
- Automatic device wipe
- Universal decryption

## Required tests

- Unknown-device inventory
- DNS anomaly
- Controlled scan quarantine live-fire
- False-positive release
- Suricata and Zeek profile conformance
- Endpoint isolation
- Sentinel-offline behavior

## Acceptance

Sentinel detects and explains controlled threats, preserves evidence, contains only within policy, and returns the network to verified safe state.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
