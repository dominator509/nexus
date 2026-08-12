# SPEC-011 - Home, Devices, Media, Appliances, Irrigation, and Robotics Providers

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define Home Assistant as primary abstraction, local fast path, state verification, device capabilities, media, appliances, lawn, vacuums, and future robots.

## Canonical terms

HomeProvider, Area, Device, Entity, DeviceCapability, FastPathIntent, StateVerification, AutomationHandoff, RobotProvider. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Home Assistant is the primary provider and source of home device state and automation truth.
2. Known commands use local edge parsing, cached policy, Home Assistant action, and observed-state verification without a generative model.
3. Every device maps to friendly identity, area, owner, provider reference, capabilities, current state, history, health, and risk.
4. Conditional commands become Temporal or Home Assistant automations based on durability and locality rules.
5. Sonos, major TVs, lighting, HVAC, vacuum, irrigation, appliances, energy, IR, and future robots expose provider-neutral capabilities.
6. Robot capabilities declare physical workspace, speed, force, safety interlocks, emergency stop, human presence, and approval class before activation.
7. Offline edge operation permits only cached low-risk capabilities and queues canonical synchronization.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Reimplementing Home Assistant integrations
- Assuming every appliance is locally controllable
- Unverified robot motion
- Cloud-only household fast path

## Required tests

- Home Assistant WebSocket integration
- Deterministic light live-fire
- Conditional workflow
- Offline operation
- State mismatch
- Capability discovery
- Robot safety schema without motion

## Acceptance

Nexus controls and verifies certified home capabilities quickly while unsupported, unhealthy, ambiguous, and high-risk functions fail closed.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
