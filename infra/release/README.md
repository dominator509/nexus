# EP-042 M3 Release Transport Infrastructure (SPEC-016, SPEC-024)

## Component record

| Field                | Value                                                                                                                                                      |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Component            | SeaweedFS 4.43 (`chrislusf/seaweedfs:4.43`)                                                                                                                |
| Digest               | `sha256:4d5118c198a6b9c0470c04b1822a0671459301625d995d1764c352bb553b3160`                                                                                  |
| License              | Apache-2.0 (verified upstream LICENSE)                                                                                                                     |
| Source               | https://github.com/seaweedfs/seaweedfs                                                                                                                     |
| Replacement contract | ArtifactStore contract; local filesystem/NAS default, R2/B2/S3 fallback                                                                                    |
| Registry             | `COMPONENT_REGISTRY.yaml` id `seaweedfs` (owner EP-037 M4)                                                                                                 |
| Certification        | PROVIDER CERTIFIED for exact exercised S3-gateway runtime/interface (EP-037 M4); EP-042 M3 adds release-transport integration proofs over the same surface |

## What this package owns

`infra/release/` is the EP-042 M3 transport boundary:

- `src/sigv4.ts` - real AWS SigV4 request signing over Web Crypto (HMAC-SHA256)
- `src/s3.ts` - minimal real S3 client over global fetch (healthz, bucket ops, object ops, list)
- `src/transport.ts` - digest-bound publish/fetch, readiness probe, idempotent publish, current-run redacted audit events
- `src/errors.ts` - typed transport errors (SPEC-006 codes)
- `src/cli.ts` - real CLI surface used by the transport scripts
- `scripts/` - real POSIX transport scripts (`release-probe.sh`, `release-publish.sh`, `release-fetch.sh`)
- `providers/seaweedfs.yaml` - provider manifest
- `containers/seaweedfs.yaml` - digest-pinned container configuration
- `fixtures/` - canonical release manifest + component artifact fixtures

## Transport invariants

- `DIGEST PRESENT != ARTIFACT VERIFIED` - fetch recomputes sha256 over real bytes and fails closed on mismatch.
- `UPDATE PLAN EXISTS != UPDATE EXECUTED` - the transport publishes and fetches bytes; it never executes an update.
- `TRANSPORT CONFIG EXISTS != TRANSPORT EXECUTED` - config validation is a separate gate step.
- `RELEASE MANIFEST EXISTS != RELEASE VERIFIED` - publishing bytes is not release verification.
- `SIGNATURE FIELD EXISTS != SIGNATURE VERIFIED` - the transport never promotes a signature field to valid.

## Certification boundary (honest)

- REAL SigV4 transport over a real SeaweedFS S3 gateway: exercised by the M3 integration suite and gate.
- Real signature verification (release component signatures): NOT ASSERTED - no key store or verifier runs in M3.
- Update execution, installer execution, canary rollout, backup/restore execution, rollback drills,
  offline bundle production, release build, deployment, remote synchronization: NOT ASSERTED.
- External clouds (R2/B2/AWS S3): NOT ASSERTED - only the local ephemeral SeaweedFS gateway is exercised.
