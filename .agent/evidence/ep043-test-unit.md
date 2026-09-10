## `scripts/test-unit.sh`
**Exit Code**: 1

### Observation
The `release-evidence` tests ran successfully after making `ep043_unit_readiness.test.ts` hermetic. However, `test-unit.sh` failed down the line due to an environment limitation (Docker `overlayfs` permissions).

### Output Snippet
```
packages/contracts test:unit:   × EP-001 generated contracts through real PostgreSQL > round-trips a NexusControlObject and ActionRequest via SQL 6036ms
packages/contracts test:unit:      → docker run -d --name nexus-ep001-ts-9dd7ca79 -e POSTGRES_USER=nexus -e POSTGRES_PASSWORD=nexus-test -e POSTGRES_DB=nexus -p 127.0.0.1::5432 postgres:18.4 failed: Unable to find image 'postgres:18.4' locally
docker: failed to extract layer (application/vnd.oci.image.layer.v1.tar+gzip sha256:130549402290405a478adedd2e14bf54504d90b21f8c1275ae05d89e93d139ba) to overlayfs as "extract-554284181-GIYA sha256:d8c9d1e0f0e4c370b18f6624610f588626f4b0dda2cc04a7263b6d94b0086a9a": failed to convert whiteout file "etc/alternatives/.wh.pager.1.gz": operation not permitted
```
Result: NOT_RUNNABLE_ENV(Docker layer extraction fails due to overlayfs permissions in the scratch workspace)
