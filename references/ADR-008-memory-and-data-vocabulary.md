# ADR-008 - Memory and Data Vocabulary

Status: Accepted
Date: 2026-08-12
Owner: hermes-nexus-main

## Context

EP-004 owns PostgreSQL durability, pgvector retrieval, memory records, world
graph abstraction, retention, and migrations (SPEC-002). The node contract
lists ten public interfaces. SPEC-002's "Canonical terms" section names
`MemoryRecord`, `MemoryProposal`, `MemoryType`, `Sensitivity`,
`RetentionPolicy`, `Provenance`, `Supersession`, `EmbeddingRef`,
`WorldGraphRepository`, `ContextCandidate`, `ContextCapsule` as vocabulary
locked. `MemoryType` already exists in `docs/vocabulary/README.md` (from the
EP-002 vocabulary, sourced from SPEC-002); the remaining names do not.
SPEC-002 and the EP-004 milestone doctrine require every new public name to
come from an accepted vocabulary or be added by an ADR and a schema update
in the same milestone.

## Decision

Add the following vocabulary-locked contracts, owned by `crates/nexus-data`
and documented in `docs/vocabulary/README.md`:

- `Sensitivity`: `PUBLIC`, `HOUSEHOLD`, `PERSONAL`, `SENSITIVE`,
  `BUSINESS_CONFIDENTIAL`, `SECURITY`, `SECRET`. Memory-record data
  classification (SPEC-002 behavior 4, SPEC-020). Wire strings match the
  canonical privacy ladder so memory filtering and redaction reuse the
  same policy classes (INV-014).
- `MemoryStatus`: `PROPOSED`, `ACTIVE`, `SUPERSEDED`, `REJECTED`, `DELETED`.
  Memory record lifecycle (SPEC-002 behaviors 5 and 8). `PROPOSED` records
  are policy-evaluated proposals, never canonical facts.
- `RetentionPolicy` + `RetentionUnit`: bounded durations (`HOURS`, `DAYS`,
  `WEEKS`, `MONTHS`, `YEARS`) plus `INDEFINITE` for legal hold / no expiry
  (SPEC-002 behavior 4, SPEC-020 retention).
- `EmbeddingRef`: versioned embedding model reference (model, dimensions,
  version) so embedding upgrades re-index without losing provenance
  (SPEC-002 behavior 2). This is a reference, never the vector payload.
- `MemoryProposal`: a proposed memory write wrapper; models cannot directly
  create canonical semantic facts (SPEC-002 behavior 5).
- `MemoryRecord` is the canonical wire record mirroring
  `schemas/memory-record.schema.json` exactly (snake_case, additional
  properties false, `content_hash` 64-hex, confidence in [0,1]).

`MemoryType` stays in `nexus-domain` (already vocabulary-locked); the new
enums live in `nexus-data` because EP-004 owns the memory/data contracts and
`nexus-domain` remains the shared lower layer.

## Evidence

- `.agent/node-contracts/EP-004.md` interface map
- `.agent/specs/SPEC-002-data-memory-fabric-search-and-world-graph.md`
  canonical terms and behaviors 2, 4, 5, 8
- `docs/vocabulary/README.md` (updated in this milestone)
- `crates/nexus-data/src/memory.rs` and unit tests `ep004_unit_*`
- `schemas/memory-record.schema.json` (bootstrap; M4 amends it)

## Alternatives rejected

- Free-form strings for sensitivity/status: lose parse-time rejection that
  the vocabulary pattern provides.
- Adding the enums to `nexus-domain`: EP-004 owns memory/data semantics;
  `nexus-domain` stays the shared lower layer.
- Dedicated graph database for the world graph: rejected by SPEC-002
  non-goals and INV-015; PostgreSQL recursive queries and adjacency tables
  are the fallback doctrine.

## Consequence

`crates/nexus-data` depends on `nexus-domain` (typed IDs, `MemoryType`,
`TenantId`) and serde only. `crates/nexus-memory` (M2) implements the ports
and the PostgreSQL/pgvector adapters. `schemas/memory-record.schema.json`
is amended in M4 to lock the enum wire values. Reversal: remove the enums,
ADR, and vocabulary entries together.
