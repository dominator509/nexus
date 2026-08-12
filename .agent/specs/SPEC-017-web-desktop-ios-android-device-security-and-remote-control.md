# SPEC-017 - Web, Desktop, iOS, Android, Device Security, and Remote Control

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define first-class clients, secure local storage, background voice, push, Bluetooth, approvals, and parity.

## Canonical terms

ClientDevice, DeviceBinding, PushEndpoint, SecureStore, RemoteSession, ApprovalPrompt, BackgroundVoice, Attestation. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. React PWA is the shared web surface; Tauri packages desktop functionality and setup; Flutter provides iOS and Android.
2. Native Swift and Kotlin modules implement passkeys, biometrics, secure enclave or keystore, attestation, background audio, Bluetooth, push, and platform integrations.
3. Clients contain no permanent universal credential and use device-bound refresh, revocation, and short-lived access tokens.
4. Mobile approval displays exact action, target, risk, external effects, cost, reversibility, requester, and expiration.
5. Remote controls use the same server capability and policy path as voice and web; no hidden mobile bypass.
6. Offline clients cache only explicitly allowed data and encrypt it with platform keys.
7. Accessibility follows platform semantics and supports captions, haptics, large text, and non-speech control.
8. Push payloads contain minimal opaque references; sensitive content is fetched after authentication.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- WebView-only security features
- Secrets in push notifications
- Different authorization semantics by client
- Mandatory app-store release for self-hosting

## Required tests

- iOS and Android integration
- Passkey and biometric
- Push privacy
- Bluetooth endpoint
- Remote action policy
- Offline cache
- Accessibility

## Acceptance

An owner can securely operate and approve Nexus from iOS, Android, web, or desktop with consistent state, policy, and privacy.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
