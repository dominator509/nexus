# EP-037 Artifact Test Material

Test zone for EP-037 ArtifactStore behavior proofs (SPEC-024). The M2
behavior suite (content addressing, hash verification, delete
absent-verification, backup/restore/migration invariants over REAL
filesystem roots) lives in `connectors/storage-local/tests/`
(`ep037_m2_local.rs`, 15 tests). This directory is the umbrella for
EP-037-owned test material; later milestones add their suites here.

Truthfulness boundaries exercised by the M2 suite:

- A write verifies the caller-supplied hash against the actual bytes
  before persisting (never trust a claimed hash).
- Every read re-hashes bytes on disk; corruption fails Verification,
  never silently succeeds.
- Delete is a ladder: DELETE_REQUESTED -> DELETE_ACCEPTED ->
  RESOURCE_ABSENT_VERIFIED (verified absence after removal).
- A backup with an unverifiable hash is rejected; duplicate backups
  Conflict.
- Restore requires every required hash verified on the fresh target
  before validation.
- Migration verifies objects on the target before any delete approval.
