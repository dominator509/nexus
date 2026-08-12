# SPEC-025 - Microbrain Dataset, Training, Evaluation, Shadow, and Promotion

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define a specialized small model pipeline that can replace reflex traffic only after objective proof.

## Canonical terms

Microbrain, TrainingExample, FrozenEval, TeacherConsensus, HardNegative, QLoRA, GGUF, ShadowDecision, PromotionGate, OutOfDistribution. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. Microbrain training is a separate service and artifact pipeline, not a runtime dependency of Nexus V1.
2. The model targets only NexusControlObject interpretation, capability selection, routing, risk, privacy, ambiguity, quoted instruction, and escalation.
3. A frozen hidden test set is created before training and never used for gradient updates or prompt iteration.
4. Training data combines deterministic generation, reviewed frontier teacher consensus, hard negatives, and explicitly opted-in scrubbed corrections.
5. Evaluation measures exact schema, intent, arguments, routing, risk, approval, injection resistance, out-of-distribution escalation, latency, memory, and quantization regression.
6. Any consequential false-positive in the protected test class is a hard promotion failure.
7. The candidate runs shadow against DeepSeek, then low-risk canary, then gradual traffic with automatic fallback.
8. Model, adapter, dataset, code, evaluation, and voice or language licenses are separately recorded.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- General assistant model
- Training from all user traffic
- Replacing deterministic policy
- Big-bang cutover
- Reasoning transcript requirement

## Required tests

- Dataset lineage
- Frozen split guard
- Training reproducibility
- Quantization comparison
- Adversarial eval
- Shadow exact-match
- Canary rollback

## Acceptance

The training factory is reproducible and safe even if no model meets promotion thresholds; DeepSeek remains the functioning V1 provider.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
