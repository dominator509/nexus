# SPEC-021 - Cameras, Frigate, go2rtc, Roku Home, Visitor Identity, and Two-Way Audio

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define camera capabilities, local stream verification, recording, event ingestion, visitor handling, Roku fallback tiers, and privacy.

## Canonical terms

CameraProvider, CameraCapability, StreamRef, ReviewItem, VisitorEvent, KnownPerson, UnknownPerson, TwoWayAudio, RokuCapabilityTier. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Frigate is the primary local NVR and object-event source; go2rtc handles verified stream normalization.
2. Roku provider performs owned-device inventory and advertises only capabilities proven through supported or observed authenticated paths.
3. Fallback order is verified local stream, authenticated vendor interface, Google Home bridge, browser automation, then unavailable.
4. Browser automation is isolated, monitored, rate-limited, and never treated as a stable API without certification.
5. Camera events include camera, time, object, zones, confidence, media references, retention, and privacy class.
6. Known-person matching is advisory and cannot unlock or disarm by itself.
7. Two-way audio requires verified speaker path, user or policy approval, disclosure rules, and echo handling.
8. Cameras live on segmented networks and cannot reach trusted workstations or data stores.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Replacing Roku firmware by default
- Claiming microSD remote read without proof
- Face match as authorization
- Public camera endpoints

## Required tests

- Frigate event ingest
- Stream health
- Roku inventory and capability negotiation
- Visitor live-fire
- Two-way audio certification
- Network isolation
- Vendor-change failure

## Acceptance

Nexus provides a truthful camera dashboard and visitor workflow while unsupported Roku functions remain visibly disabled.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
