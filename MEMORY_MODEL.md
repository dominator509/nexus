# MEMORY MODEL

## Types

- Working: task-local and expires automatically.
- Episodic: events and interactions with temporal order.
- Semantic: durable facts and preferences.
- Entity: person, household, business, device, project, account, or place knowledge.
- Procedural: successful repeatable methods and workflow outcomes.
- Decision: chosen alternatives, rationale, evidence, and supersession.
- Skill: installed skill metadata, performance, and permissions.
- System: deployment, incidents, component health, and operational history.

## Required metadata

Every memory has owner namespace, tenant, subject references, source, actor, created and observed time, confidence, sensitivity, purpose, retention policy, legal hold, derived-from references, supersedes reference, embedding model version, content hash, and deletion state.

## Write policy

Models and agents submit `MemoryProposal`. Deterministic policy decides accept, request review, merge, supersede, retain as working only, or reject. Private and adult content defaults to no durable memory unless explicitly enabled. Authentication secrets never enter semantic memory.

## Retrieval

Context Engine performs structured filters and authorization first, then full-text, vector, graph, recency, importance, task relevance, confidence, and diversity ranking. It emits a minimized context capsule with citations to memory IDs. A model never receives unrestricted database search.

## Consolidation

Temporal workflows group episodes, detect contradictions, create candidate semantic facts, apply retention, regenerate embeddings after model changes, and preserve old versions until the supersession window closes. Human corrections outrank model-derived facts.
