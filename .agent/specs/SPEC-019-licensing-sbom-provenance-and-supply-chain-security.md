# SPEC-019 - Licensing, SBOM, Provenance, and Supply-Chain Security

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define acceptable licenses, sidecar isolation, model and data licensing, version locks, signing, attestations, scanning, and advisory response.

## Canonical terms

LicenseClass, SBOM, Provenance, Attestation, ArtifactDigest, SourceOffer, Waiver, Advisory, SidecarBoundary. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Every source, package, image, model, dataset, firmware, and generated asset has version, digest, license, provenance, owner, and replacement boundary.
2. Permissive components may be embedded after review; MPL and LGPL require obligation analysis; GPL and AGPL default to process or appliance isolation; noncommercial artifacts are prohibited.
3. Model code, weights, training data, voices, and output rights are assessed separately.
4. Every release includes SPDX or CycloneDX SBOMs, vulnerability reports, license report, source notices, and signed provenance.
5. GitHub Actions are pinned by immutable commit SHA and use minimum permissions.
6. OCI images are pinned by digest in release manifests and scanned before promotion.
7. Advisories create incidents and bounded remediation workflows; critical exploitable findings block release.
8. A waiver has owner, exact version, reason, controls, expiry, and replacement plan.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Assuming open source means unrestricted resale
- Floating latest tags
- Ignoring transitive model licenses
- Permanent waivers

## Required tests

- License-policy fixtures
- SBOM completeness
- Unsigned image rejection
- Tampered artifact rejection
- Noncommercial wake model rejection
- Advisory workflow

## Acceptance

The release evidence can answer what every shipped byte is, where it came from, under what terms, and how it was verified.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
