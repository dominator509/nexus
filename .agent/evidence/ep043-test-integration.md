## `scripts/test-integration.sh`
**Exit Code**: 1

### Observation
Failed because it requires Minio endpoint but the environment doesn't have it set.

### Output Snippet
```
failures:
    ep037_integration_s3_compatible_corruption_detected
    ep037_integration_s3_compatible_delete_and_absent
    ep037_integration_s3_compatible_put_get_digest_verified

test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

thread 'ep037_integration_s3_compatible_corruption_detected' panicked at tests/backup/src/lib.rs:282:53:
NEXUS_MINIO_ENDPOINT must be set: NotPresent
```
Result: NOT_RUNNABLE_ENV(NEXUS_MINIO_ENDPOINT not present)
