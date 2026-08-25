# EP-042 M5 Operations - Offline Bundle, Release, Install, Rollback

Every command below really exists in this repository and is exercised by
the EP-042 M5 gate unless marked otherwise. Run from the repository root.

## Health / readiness

```sh
# EP-044 control plane (canonical runtime smoke; mandatory since EP-044 DONE)
NEXUS_SMOKE_URL=http://127.0.0.1:8443 sh scripts/smoke/runtime.sh
#   -> runtime smoke: ok   (healthz healthy, readyz true, capabilities non-empty)

# Start the control plane if absent
NEXUS_SMOKE_URL=http://127.0.0.1:8443 sh scripts/local-start.sh core
#   -> local start core: ok

# Release transport readiness (M3; requires a running S3-compatible gateway)
sh infra/release/scripts/release-probe.sh
```

## Release transport (M3)

```sh
# Publish a release manifest + component artifacts to the transport
export NEXUS_RELEASE_S3_ENDPOINT=... NEXUS_RELEASE_ACCESS_KEY=...
export NEXUS_RELEASE_SECRET_KEY=... NEXUS_RELEASE_BUCKET=...
export NEXUS_RELEASE_RUN_ID=... NEXUS_RELEASE_GIT_COMMIT=...
sh infra/release/scripts/release-publish.sh <release-id> <manifest.json> <artifacts-dir>

# Fetch a published release (digest-bound; fails closed on mismatch)
sh infra/release/scripts/release-fetch.sh <release-id> <manifest-out> <components-out> <compIds>
```

## Installer (M4, local transactional install)

```sh
sh installers/scripts/installer-install.sh <install-root> <release-id> <install-id> \
  <manifest.json> <artifacts-dir> <componentId=relPath[,...]>
sh installers/scripts/installer-status.sh <install-root> <release-id> <install-id>
sh installers/scripts/installer-recover.sh <install-root> <release-id> <install-id>
sh installers/scripts/installer-rollback.sh <install-root> <release-id> <install-id> <backup-digest>
```

## Offline bundle (M5)

```sh
# Produce a REAL bundle from real files
sh offline-bundle/scripts/bundle-produce.sh <bundle-dir> <bundle-id> <release-id> \
  <manifest.json> <componentId=artifactPath:kind[,...]> \
  [sboms.csv] [licenses.csv] [migrations.csv] [recovery.csv]

# Verify a bundle (digest-bound; every denial exits nonzero typed)
sh offline-bundle/scripts/bundle-verify.sh <bundle-dir>

# OFFLINE install from a verified bundle (NO transport required)
sh offline-bundle/scripts/bundle-install.sh <bundle-dir> <install-root> \
  <release-id> <install-id> <componentId=relPath[,...]> <run-id> <git-commit>

# Rollback drill (receipt written only after verified restoration)
sh offline-bundle/scripts/bundle-rollback.sh <install-root> <release-id> <install-id> \
  <backup-digest> <absPath=priorBytes[,...]> <run-id> <git-commit>
```

## Backup / restore

Backup-before-update is enforced by the installer surface: every install
into an existing root creates a real backup (real bytes + real sha256
digest) and a backup failure denies the update. Restore is the rollback
path: `installer-rollback.sh` (M4) or `bundle-rollback.sh` (M5) restores
the exact prior bytes and verifies them before writing any receipt.

## Upgrade / disable / abort

- Upgrade: produce/verify a release, then `installer-install.sh` (M4) or
  `bundle-install.sh` (M5). Both are transactional: staged replacement,
  digest validation, atomic switch, verification.
- Disable/abort: installs accept an abort signal; a cancelled install
  leaves the old state valid and removes staged state.
- Rollback: `installer-rollback.sh` / `bundle-rollback.sh`.

## Troubleshooting

- A `BUNDLE_DIGEST_MISMATCH` denial means a bundle file changed after
  production - regenerate the bundle, never edit payloads in place.
- A `BUNDLE_MISSING_FILE` denial means the bundle is incomplete -
  re-run `bundle-produce.sh`.
- A `PATH_ESCAPE` denial means the bundle contains a hostile path -
  treat the bundle as untrusted and discard it.
- A `WRONG_RELEASE_ID` denial means the bundle manifest and release
  manifest disagree - regenerate from the matching release.
- `RESOURCE_EXHAUSTION`: before heavy fixture/container work, check
  `df -P /`; a full host disk is an environment event, not installer
  proof. Reclaim only owned fixtures (`docker rm -f nexus-ep042-*`,
  remove `/tmp/nexus-ep042-m5-*`); never a global prune.

## Certification boundary (honest)

- INTERNAL BEHAVIOR CERTIFIED for the exact isolated local surfaces
  exercised by the M5 gate: bundle production from real files, digest-
  bound bundle verification (all denial classes), offline install from a
  local bundle with transport absent, rollback drill with receipt after
  verified restoration, current-run redacted evidence.
- NOT ASSERTED: production host upgrade, real release signature
  verification (SIGNATURE PRESENT != SIGNATURE VALID), canary rollout,
  production backup/restore, production rollback, deployment, AWS/R2/B2
  transport, arbitrary production environments.
