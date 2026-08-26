# ROLLBACK

## Triggers

Failed smoke, elevated errors or latency, identity or policy regression, action verification mismatch, data corruption, provider incompatibility, cache-cost spike with quality regression, security advisory, failed edge compatibility, update stall, or operator decision.

## Owner

The release operator owns application rollback. Database, identity, secret, policy, firewall, and data rollback require the named specialist and approval class in the release manifest.

## Application

Stop promotion, preserve evidence, route traffic to prior signed release, restart compatible workers, verify database compatibility, run smoke and critical live-fire, and announce status.

## Database

Prefer forward-compatible old application on expanded schema. If restore is necessary, stop writes, preserve current state, restore encrypted verified backup to a new database, validate manifests and event offsets, switch atomically, and retain both copies until closure.

## Configuration and policy

Configuration, OpenFGA models, OPA bundles, connector manifests, skills, and provider routes are versioned. Restore the last signed version and invalidate related caches and capability tokens.

## Home edge and mobile

Rollout rings preserve the prior package. Edge automatically returns to the previous image after failed health. Mobile feature flags and server compatibility prevent forced immediate store rollback.

## Verification

Identity login, policy decision, API, NATS, Temporal, DeepSeek reflex, home edge, backup, and affected provider proofs pass. Record duration, data loss, user impact, and follow-up incident.

## Release evidence rollback (EP-043 M5)

This section is owned by EP-043 and covers rollback of the release evidence produced by the production readiness engine (`release-evidence/`).

### Eligibility and triggers

Rollback of release evidence is eligible when any of these hold:

- the readiness report was generated from a wrong or stale commit,
- the release manifest is tampered, malformed, or unverifiable,
- an artifact digest does not match the manifest,
- a bad release was applied (wrong `release_id`, forged report, corrupted bytes),
- the ship gate regressed from an inspected state,
- an operator decision requires returning to the last verified evidence.

Evidence rollback is bounded to the release evidence surface: `PRODUCTION_READINESS.md` (committed) and `dist/release/RELEASE_MANIFEST.json` (generated). It never rolls back the control plane, database, identity, or policies; those follow the sections above.

### Rollback procedure

The rollback drill is a real command:

```sh
sh scripts/ep043-rollback-drill.sh
```

The drill runs in a throwaway clone of the candidate commit and performs:

1. capture state A: sha256 of the committed `PRODUCTION_READINESS.md` and the canonical manifest component digests,
2. apply state B: a forged READY report and corrupted manifest (isolated, never in the working tree),
3. verify B differs from A,
4. execute rollback: `git restore` the committed report and regenerate the manifest from canonical state,
5. verify A exactly: restored report sha256 equals captured A and regenerated component digests equal captured A,
6. verify the restored manifest with the verify-manifest CLI,
7. write dated drill evidence only after verification.

### Verification after rollback

After any evidence rollback, an operator must run:

```sh
node --experimental-transform-types --import file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs release-evidence/src/cli.ts verify-manifest --manifest dist/release/RELEASE_MANIFEST.json
node --experimental-transform-types --import file://$(pwd)/release-evidence/scripts/ts-resolve-loader.mjs release-evidence/src/cli.ts readiness --output PRODUCTION_READINESS.md
```

The report must regenerate bound to the exact current commit, and verify-manifest must report `ok`. A receipt written before verification is not evidence.

### Failure classification

Rollback fails closed on: missing rollback source, corrupt source, wrong release identity, wrong commit identity, missing backup, unverified backup, digest mismatch, rollback command failure, post-rollback verification failure, stale rollback evidence, forged rollback receipt, duplicate or conflicting rollback state, cancellation, timeout, and foreign target paths. `ROLLBACK ATTEMPTED` is never `ROLLBACK VERIFIED`.

### Drill evidence

The drill writes `ep043-drill-rollback-<ts>.md` under `.agent/state/evidence/`, binding run_id, candidate commit, captured state A, applied state B, the executed rollback, and the verified restoration. The readiness engine reads this evidence to satisfy the rollback drill acceptance obligation.
