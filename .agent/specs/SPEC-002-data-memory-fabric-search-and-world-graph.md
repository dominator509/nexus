# SPEC-002 - Data, Memory Fabric, Search, and World Graph

Status: Accepted blueprint specification
Owner: Nexus Architecture Council
Generated: 2026-08-12

## Goal

Define PostgreSQL durability, pgvector search, memory types, provenance, graph abstraction, retention, and future graph-engine substitution.

## Canonical terms

MemoryRecord, MemoryProposal, MemoryType, Sensitivity, RetentionPolicy, Provenance, Supersession, EmbeddingRef, WorldGraphRepository, ContextCandidate, ContextCapsule. These names are vocabulary locked. A new synonym requires an ADR and schema update.

## Required behavior

1. PostgreSQL is canonical for control-plane state, policy receipts, memory metadata, objectives, tasks, artifacts, connector bindings, and audit.
2. pgvector stores initial embeddings; embedding model and dimensions are versioned per row.
3. Memory types are working, episodic, semantic, entity, procedural, decision, skill, and system.
4. Every memory record has provenance, actor, observed time, confidence, sensitivity, purpose, retention, derived-from, supersedes, content hash, and deletion state.
5. Memory writes are proposals evaluated by policy; models cannot directly create canonical semantic facts.
6. Retrieval combines authorization filters, structured lookup, full-text, vector, graph, recency, importance, confidence, and diversity.
7. WorldGraphRepository is a port with PostgreSQL implementation first; a future graph engine is a projection or implementation behind the same contract.
8. Export, deletion, legal hold, retention, and re-embedding workflows are durable and audited.

## Inputs and outputs

Inputs and outputs use canonical JSON Schemas under `schemas/`, generated language bindings, authenticated tenant and principal context, and versioned event contracts. Free-form provider payloads are normalized at the infrastructure boundary and never become domain contracts.

## Error states

All failures use SPEC-006 codes, preserve correlation, redact sensitive content, and distinguish validation, authentication, authorization, policy, unavailable, timeout, conflict, rate limit, external provider, verification, compensation, and internal invariant failures.

## Security and privacy

SECURITY.md, SPEC-005, and SPEC-020 are binding. Least privilege, data classification, purpose limitation, egress policy, audit, and fail-closed behavior apply to every requirement.

## Non-goals

- External memory SaaS as canonical store
- Embedding secrets
- Unbounded prompt history
- Graph vendor SDK in domain or application

## Required tests

- Real PostgreSQL migration and isolation tests
- Vector version migration test
- Memory policy tests
- Hybrid retrieval benchmark
- Graph repository contract suite
- Export and deletion live-fire

## Acceptance

A fresh database can migrate, ingest, retrieve, supersede, export, delete, restore, and re-embed memory without losing provenance or tenant isolation.

## Traceability

The validation matrix in TESTING.md maps each numbered behavior to implementation tests, live-fire proofs, provider certification, or hardware certification. No requirement may be marked complete from documentation review alone.
