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
