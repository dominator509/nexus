# SPEC-016 - Deployment Profiles, Setup, Compute Fabric, Provisioning, and Updates

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define one distribution, placement, cloud adapters, offline bundle, secure bootstrap, signed updates, and managed or self-hosted parity.

## Canonical terms

DeploymentProfile, NodeManifest, WorkloadManifest, PlacementPlan, Provisioner, BootstrapToken, ReleaseManifest, OfflineBundle, UpdateTransaction. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. One signed Nexus distribution supports managed, BYOC, existing SSH, hybrid, and fully local profiles.
2. OpenTofu and cloud-init provision Contabo, Hetzner, DigitalOcean, AWS, and generic SSH through provider adapters.
3. Provider credentials remain in the local setup process or short-lived OAuth and are discarded after provisioning unless infrastructure management is enabled.
4. Compute Fabric profiles node hardware, trust, locality, power, availability, and latency, then places workloads according to manifests.
5. The offline bundle contains signed images, models, manifests, licenses, SBOMs, migrations, and recovery tools.
6. Updates verify signatures, back up state, apply compatible migrations, canary, observe, promote, or automatically roll back.
7. Release channels are stable, beta, developer, and pinned. Security updates may override a paused feature update with explicit disclosure.
8. Kubernetes is optional and cannot become a dependency of household deployment.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- Cloud-only control plane
- Permanent cloud master credentials
- Manual container orchestration for ordinary users
- Kubernetes-first home

## Required tests

- OpenTofu plan tests
- Cloud-init lint
- Local profile install
- Existing SSH install
- Offline install
- Update rollback
- Placement policy
- Credential deletion

## Acceptance

Nexus Setup can reproducibly create, update, repair, restore, and migrate a deployment while preserving identity and without hidden vendor dependency.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
