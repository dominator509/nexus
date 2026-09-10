## `scripts/live-fire.sh`
**Exit Code**: 1

### Observation
Similar to the unit tests, the live fire tests failed while trying to pull and extract the PostgreSQL Docker image.

### Output Snippet
```
EP-035 M5 gate: FAIL - vitest run failed
docker: failed to extract layer (application/vnd.oci.image.layer.v1.tar+gzip sha256:130549402290405a478adedd2e14bf54504d90b21f8c1275ae05d89e93d139ba) to overlayfs as "extract-870076985-1kgd sha256:d8c9d1e0f0e4c370b18f6624610f588626f4b0dda2cc04a7263b6d94b0086a9a": failed to convert whiteout file "etc/alternatives/.wh.pager.1.gz": operation not permitted
```
Result: NOT_RUNNABLE_ENV(Docker layer extraction fails due to overlayfs permissions)
