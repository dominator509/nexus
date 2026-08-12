# SPEC-014 - Email, Phone, Fax, Notifications, and Communications Routing

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define canonical communication objects, providers, governed sends, phone media, fax lifecycle, private notification routing, and legal policy.

## Canonical terms

CommunicationRouter, Mailbox, Thread, Message, Draft, CallSession, CallLeg, FaxJob, Notification, Channel, DeliveryReceipt, DisclosurePolicy. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Email provider interface supports Gmail, Microsoft Graph, IMAP and SMTP, and self-hosted mail without vendor-specific domain logic.
2. Read, draft, send, reply, forward, archive, label, and attachment capabilities have separate scopes.
3. Asterisk 22 LTS is the telephony gateway; SIP carriers are providers. Audio streams through the Nexus voice pipeline over a secure media bridge.
4. Calls are durable workflows with objective, participant, disclosure, consent, interruption, escalation, summary, transcript policy, and cost cap.
5. ICTFax is primary self-hosted control; HylaFAX is compatibility; Telnyx and Phaxio are external fallbacks.
6. Fax jobs preserve source artifact hash, number normalization, pages, carrier, retries, status, inbound route, and archive.
7. Communication Router selects push, desktop, speaker, SMS, email, phone, watch, car, or future robot based on person, presence, privacy, urgency, quiet hours, cost, and availability.
8. External sends at R2 or higher require policy; crisis, legal, financial, mass-send, and reputation messages require stronger approval.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Free PSTN promise
- Undisclosed AI call where prohibited
- Unbounded robocalling
- Reading all mail with a social agent token

## Required tests

- Real email lifecycle
- Asterisk test call
- Carrier failure and fallback
- Fax lifecycle
- Quiet-hour escalation
- Private notification routing
- Jurisdiction policy table

## Acceptance

Certified communication providers complete real end-to-end delivery with permission, disclosure, receipts, cost, audit, and safe failure.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
