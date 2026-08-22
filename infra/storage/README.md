# EP-037 Storage Infrastructure Root

Provider-neutral storage topology for SPEC-024 ArtifactStore. All backends
(local filesystem, NAS, SeaweedFS, MinIO compatibility, Cloudflare R2,
Backblaze B2, Amazon S3) satisfy ONE contract defined in
`crates/nexus-artifacts` (`ArtifactStore` port).

## Backend adapter ownership

| Backend | Adapter root (later milestones) | Status |
| --- | --- | --- |
| Local filesystem | `connectors/storage-local/` (M2) | CONTRACT BOUNDARY ONLY |
| NAS | `connectors/storage-nas/` (M3) | CONTRACT BOUNDARY ONLY |
| SeaweedFS | `connectors/storage-seaweedfs/` (M4) | CONTRACT BOUNDARY ONLY |
| S3-compatible (S3, MinIO, R2, B2) | `connectors/storage-s3/` (M5) | CONTRACT BOUNDARY ONLY |

MinIO is compatibility-only because the community repository is archived;
the UI warns and recommends a maintained alternative (SPEC-024
requirement 2).

## Truthfulness boundaries

- A backend declaration is not a benchmark.
- An artifact written is not an artifact verified.
- A backup created is not a restore proven.
- Encryption metadata is not a key; the recovery key lives outside the
  storage backend and outside the backup.
- Migration deletes old objects only after hash verification and human
  approval (SPEC-024 requirement 8).

This directory intentionally contains only topology documentation at M1.
Backend-specific configuration lands with the adapter milestones.
