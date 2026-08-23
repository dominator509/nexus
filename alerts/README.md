# alerts/ - EP-038 M1 Alert Contract & Configuration

Status: **CONTRACT/CONFIG CERTIFIED (M1)** - no provider backend is
claimed, configured, or delivered in M1. PagerDuty/Slack/email/OTel
alert delivery are **NOT ASSERTED** until later milestones.

## Scope

This directory holds only M1-owned alert material:

- `catalog.yaml` - canonical alert rule vocabulary (deny-unknown ids,
  severities, thresholds, owners, runbooks, suppression, test-signal,
  resolution semantics)
- `redaction-policy.yaml` - fail-closed alert redaction contract
- `slo-catalog.yaml` - canonical SLO definitions feeding the
  `SloEvaluator` contract

## Contract invariants (SPEC-007 alerts)

1. Every alert has: stable id, owner, severity, threshold, runbook,
   suppression policy, test signal, and resolution state.
2. Alert bodies never contain raw secrets, prompts, tokens, or private
   payloads. Redaction runs before any egress.
3. `NO ALERTS != SYSTEM HEALTHY`. Absence of alert traffic is not
   evidence of health; health is derived from observed checks.
4. Repeated identical failures dedupe (IncidentSink) without hiding
   severity escalation.
5. Zero-denominator SLOs are `NoData`/`InsufficientEvidence`, never
   green; alerts must not fire "healthy" on missing data.

## Files

- `catalog.yaml` - alert rule catalog (M1-owned canonical rules)
- `redaction-policy.yaml` - redaction policy for alert payloads
- `slo-catalog.yaml` - SLO catalog bound to SloEvaluator contracts

Validation is performed by `scripts/ep038-m1-tests.sh` (rule id
deny-unknown, severity vocabulary, redaction fail-closed).
