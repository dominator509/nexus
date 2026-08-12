# PRODUCTION READINESS

A checkbox is completed only by observed evidence from the named command or artifact in the current release candidate.

## Functional

- [ ] Every required row in LIVE_FIRE_PROOFS.md passes under `NEXUS_REQUIRE_ALL_PROOFS=1 sh scripts/live-fire.sh`.
- [ ] Every accepted specification requirement is mapped and green in TESTING.md and `tests/traceability/`.
- [ ] Optional providers shown as certified have current evidence in `provider-certification/RESULTS.md`.
- [ ] Hardware shown as supported has current evidence in `hardware/COMPATIBILITY_MATRIX.md`.
- [ ] Non-goals remain excluded by `sh scripts/scope-gate.sh`.

## Testing and reality

- [ ] `sh scripts/verify.sh` prints `verify: ok` in a fresh clone equivalent.
- [ ] `sh scripts/reality-gate.sh` prints `reality gate: ok`.
- [ ] Required CI has no ignored, warning-only, or continue-on-error gate.
- [ ] Regression coverage includes every live-fire outcome and every prior critical incident.

## Security and privacy

- [ ] `sh scripts/security-check.sh` prints `security check: ok`.
- [ ] `sh scripts/dependency-audit.sh` prints `dependency audit: ok`.
- [ ] `sh scripts/license-gate.sh` prints `license gate: ok`.
- [ ] Threat model, trust boundaries, OpenFGA model, OPA policies, and step-up matrix are reviewed.
- [ ] No secret, prompt, raw audio, private image, or customer content appears in logs or support bundles.
- [ ] Export, deletion, retention, backup, and API egress are demonstrated.
- [ ] No unvalidated compliance claim appears in product copy.

## Performance and resilience

- [ ] Reference control-plane load meets OBSERVABILITY.md SLOs.
- [ ] Certified home-edge hardware meets local command and voice targets.
- [ ] DeepSeek cache replay and staging traffic meet the 0.97 token-hit target.
- [ ] Provider failover, internet loss, edge loss, NATS loss, Temporal worker loss, and database restart are tested.
- [ ] Restore, update rollback, identity recovery, and Sentinel containment drills have dated evidence.

## Accessibility

- [ ] Web and desktop WCAG 2.2 AA automated and manual core-flow checks pass.
- [ ] iOS and Android VoiceOver or TalkBack, large text, switch or keyboard, captions, and haptics pass.
- [ ] Critical flows remain available without speech and without color-only status.

## Observability and operations

- [ ] Dashboards load from deployment manifests.
- [ ] Synthetic alerts reach the right owner and link the correct runbook.
- [ ] Support bundle is encrypted, previewable, and redacted.
- [ ] Incident, backup, restore, certificate, secret, provider, and update runbooks are current.

## Deployment and release

- [ ] Signed OCI, desktop, mobile, offline, SBOM, provenance, license, and release manifests exist.
- [ ] Setup wizard deploys local, existing SSH, and one BYOC provider profile.
- [ ] Staging install, upgrade, backup, restore, and rollback pass.
- [ ] Exact manual production command is generated but not executed.
- [ ] Human release sign-off records owner, date, version, evidence hash, and accepted risks.

The only passing command is `sh scripts/production-readiness-check.sh`, which must print `production readiness: ok`.
