# ADR-028 - Deployment, Release, Update, and Rollback Vocabulary

Status: Accepted
Date: 2026-08-25
Owner: EP-042 (Deployment, Release, Update, and Rollback)

## Context

SPEC-016 defines one signed Nexus distribution supporting managed, BYOC,
existing SSH, hybrid, and fully local profiles; offline bundles containing
signed images, models, manifests, licenses, SBOMs, migrations, and
recovery tools; updates that verify signatures, back up state, migrate,
canary, observe, promote, or automatically roll back; and release
channels stable, beta, developer, and pinned. SPEC-024 defines the
ArtifactStore and its manifests. The EP-042 node contract names eight
public interfaces: ReleaseManifest, SignedComponent, CompatibilityMatrix,
UpdatePlan, CanaryRing, RollbackReceipt, OfflineBundle, and
ManualPromotion.

None of these vocabulary classes existed in `crates/nexus-domain` or a
release crate. EP-042 owns the release lifecycle contracts and must encode
several authority distinctions the owner directive requires: a signature
field existing on a component is not a valid signature; an update plan
existing is not an executed update; a canary observing is not a promoted
release; a promotion decision is not a deployment; a rollback receipt
requires a backup reference; and an offline bundle existing is not an
offline bundle verified. Production promotion must remain an exact manual
action.

## Decision

Add the EP-042-owned vocabulary in `crates/nexus-release` (vocabulary
module) with unknown-value rejection at parse time and serde
deny-unknown-fields on every record:

- `DeploymentProfileMode`: `MANAGED`, `BYOC`, `EXISTING_SSH`, `HYBRID`,
  `FULLY_LOCAL`. Mirrored from the canonical
  `schemas/deployment-profile.schema.json` `mode` enum so the Rust
  contract surface cannot drift from the schema.
- `ReleaseChannel`: `STABLE`, `BETA`, `DEVELOPER`, `PINNED`. Mirrored
  from the canonical schema `release_channel` enum.
- `SignatureAlgorithm`: `ED25519` only. Anything else is rejected, never
  silently accepted.
- `SignatureState`: `UNVERIFIED`, `PRESENT`, `VALID`, `INVALID`.
  Presence is a ladder rung, not a verification claim.
- `UpdateStepKind`: `BACKUP`, `MIGRATE`, `CANARY`, `OBSERVE`,
  `ROLLBACK`. Deliberately NO `PROMOTE`: plans cannot promote.
- `UpdateState`: `PLANNED`, `PENDING`, `IN_PROGRESS`, `OBSERVING`,
  `READY_TO_PROMOTE`, `ROLLED_BACK`, `FAILED`. `READY_TO_PROMOTE` is the
  furthest an update engine may take a canary.
- `CanaryVerdict`: `OBSERVING`, `READY_TO_PROMOTE`, `ROLLBACK`.
  Deliberately NO `PROMOTED`: canaries never promote.
- `VerificationState`: `UNVERIFIED`, `VERIFIED`, `MISMATCH`, `MISSING`.
- `BundleKind`: `IMAGE`, `MODEL`, `LICENSE`, `SBOM`, `MIGRATION`,
  `RECOVERY_TOOL`.
- `RollbackState`: `REQUIRES_BACKUP`, `BACKUP_VERIFIED`,
  `ROLLBACK_VERIFIED`, `FAILED`.
- `PromotionState`: `LOCKED`, `AWAITING_HUMAN_APPROVAL`,
  `APPROVED_MANUAL_ONLY`.

All vocabulary names are new public names. They are added by this ADR and
encoded in the `nexus-release` crate as deny-unknown enums (serde derived
Deserialize rejects unknown variants; FromStr rejects unknown strings).
No cross-language JSON Schema change is required in M1 because every
public interface in the EP-042 node contract is owned by the Rust
`nexus-release` crate; the existing `deployment-profile.schema.json`
already canonicalizes the mode and release_channel enums that the crate
mirrors.

## Consequences

- Release manifests, update plans, canary rings, rollback receipts,
  offline bundles, and promotion records are provider-neutral Rust
  contracts; no provider name (MinIO, S3, R2, B2, SeaweedFS, Docker,
  Kubernetes) becomes a domain type. ObjectRef.backend remains a
  free-form string per ARCHITECTURE.md forbidden moves.
- A signature is `PRESENT` at construction and can never self-certify as
  `VALID` inside M1 (no key store or verifier exists in M1).
- An update plan without a backup first step is rejected at construction
  (backup-before-update); a rollback receipt without a backup reference
  cannot be constructed (the field is mandatory, not optional).
- Promotion requires a human approval reference; the gate returns only
  LOCKED, AWAITING_HUMAN_APPROVAL, or APPROVED_MANUAL_ONLY and never
  returns a deployment action.
- Reversal: if a later node needs an additional step kind, channel, or
  state, it must add it by ADR + vocabulary update in the same milestone.
- Security impact: redaction-first error surface; signature key material
  and secret-shaped values never appear in error messages.
- License impact: none (MIT crate, no new third-party dependency).
- Compatibility impact: `schema_version` is fixed at 1 for M1; versioned
  serialization preserves and rejects schema_version on roundtrip.
