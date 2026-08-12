# SPEC-024 - Artifacts, Object Storage, Backup, Restore, and Disaster Recovery

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define ArtifactStore, local and cloud backends, encryption, versioning, backup sets, restore, provider migration, and recovery objectives.

## Canonical terms

ArtifactStore, ObjectRef, ArtifactManifest, BackupSet, RecoveryKey, RestorePlan, RPO, RTO, StorageMigration. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. ArtifactStore supports local filesystem, NAS, SeaweedFS, MinIO compatibility, Cloudflare R2, Backblaze B2, and Amazon S3 behind one contract.
2. MinIO is compatibility-only because the community repository is archived; the UI warns and recommends a maintained alternative.
3. Artifact metadata, hash, content type, size, owner, data class, retention, encryption, version, lineage, and backend location remain canonical in PostgreSQL.
4. Sensitive artifacts are encrypted client-side or before provider upload using keys outside the storage backend.
5. Backups include databases, identity configuration, policies, workflows, memory, skills, connectors, manifests, audit, and optional artifacts according to profile.
6. Every backup has signed manifest, hashes, encryption metadata, application and schema versions, and restore compatibility.
7. Restore occurs to a fresh target, validates all components, and reconnects edge nodes through controlled re-enrollment or preserved trust.
8. Provider migration copies, verifies, changes canonical location, observes, and deletes old objects only after approval.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Backup without restore proof
- Storing recovery key beside backup
- Assuming S3 implementations are identical
- Deleting old provider first

## Required tests

- Backend contract suite
- Encrypted artifact
- Backup and fresh restore live-fire
- Corrupt backup rejection
- Storage migration
- RPO and RTO measurement

## Acceptance

A destroyed control-plane host can be replaced from encrypted backup and pass all critical smoke within the documented objective.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
