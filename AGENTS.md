# AGENTS - NEXUS CONTROL PLANE

## 1. Mission

Build Nexus as a secure, self-hosted-first Life and Business OS using the locked architecture and the deterministic 44-node graph. Finish real features through verified open-source engines and provider contracts without drift, fabricated success, hidden cloud dependency, or unsafe authority.

## 2. The boot sequence

PRIME-BLOCK-BEGIN
This repository is governed by a 6LAYER blueprint pack. AGENTS.md is the authoritative control plane; if anything here conflicts with AGENTS.md, AGENTS.md wins.
On every session start, execute THE BOOT SEQUENCE:
1. Read AGENTS.md fully. 2. Read COMMANDS.md. 3. Read .agent/GRAPH.md and .agent/LOOPS.md. 4. Run: sh scripts/ledger.sh tail 30. 5. Run: sh scripts/preflight.sh - it MUST print "preflight: ok"; if it fails, report the exact missing items from PREFLIGHT.md and stop (this is the only legitimate pre-run stop). 6. Run: sh scripts/graph-next.sh and dispatch on its one-line output exactly as .agent/GRAPH.md specifies. 7. Repeat step 6 after every completed node until ALL_DONE, then run the ship gate in AGENTS.md.
Hard rules: do not ask the user questions; choose the smallest reversible option, record it, continue. Use only commands from COMMANDS.md. Never invent an API, route, table, flag, or env var - verify in-repo or transcribe from the pack. One node at a time; milestones in order; commit after every milestone; append ledger events as .agent/LOOPS.md requires. Bounded retries per .agent/LOOPS.md - never repeat a failed fix. No mocks, stubs, demo modes, or placeholder code in production paths; scripts/reality-gate.sh and scripts/live-fire.sh must genuinely pass. Never weaken a gate, skip a test, or claim an unrun result. Stop only at NODE_BLOCKED (with the full evidence report) or ALL_DONE.
PRIME-BLOCK-END

## 3. Source-of-truth hierarchy

Current explicit user instruction > AGENTS.md and L1 laws > accepted specs and L2 architecture > fixed graph > active ExecPlan > repository code and tests > gate output as fact > append-only ledger as history. When code contradicts a spec, code changes. A spec change requires evidence, ADR, compatibility analysis, and ledger entry.

## 4. Graph protocol

One node, one lease, one writer. Use `scripts/graph-next.sh`. Execute milestones in order, heartbeat every fifteen minutes and after every milestone, commit every milestone, release a lease if stopping, and never work around a BLOCKED node. A node is DONE only after every milestone, node verify sentinel, expected-files audit, NODE_DONE ledger event, and green tag.

## 5. Stop conditions

Stop only when: preflight fails before the run; an action would destroy user or production data or cause an unspecified irreversible effect; a legal, financial, or security judgment is unanswered by the specs; the bounded ladder ends in NODE_BLOCKED with full evidence; or production deployment is requested because auto-deploy is not authorized. Do not ask the user for next steps, preferences, or confirmation. Proceed with the smallest reversible specified option in every other case.

## 6. Anti-drift

Re-read the milestone, non-goals, and ledger tail before work. Change only listed files. Revert unrelated paths immediately unless a Decision Log entry justified them before keeping them. No refactor safari, dependency swap, rename, reorganization, or cleanup outside the node.

## 7. Anti-hallucination

Never invent package APIs, routes, tables, flags, environment variables, provider behavior, device capability, model license, command, or schema. Verify in repository or authoritative upstream material. Commands come only from COMMANDS.md. Unknown external behavior remains unavailable until proved.

## 8. Anti-fixation

Use the ladder in .agent/LOOPS.md. One hypothesis and smallest fix, then narrower diagnostic, then declared fallback, then rollback, then blocked. Do not apply the same fix twice. Do not delete tests or weaken gates.

## 9. Reality law

Software that appears to work is a failure state. Only software proven by live-fire counts. Optional integrations compile and pass contracts before certification, but are not advertised as operational until real provider or hardware live-fire evidence exists. A gate passes only when run now and its sentinel is observed.

## 10. Dependencies

Prefer existing locked components. Add a dependency only when no selected component or standard library path satisfies the contract. Pin exact version and digest, record license and security, add ADR, update install and audit, and preserve a replacement boundary.

## 11. Files and commits

Follow the checkpoint protocol. Never leave a milestone uncommitted. Generated files come from canonical schemas. Never edit state outside the append-only ledger and ExecPlan progress sections.

## 12. Testing

TESTING.md is binding. Real dependencies and test-double zones are explicit. Required tests and live-fire cannot be ignored, retried until green, or made informational.

## 13. Documentation layers

L1 laws are immutable during a run. L2 specs change only by the spec-update rule. L3 graph is fixed. L4 plans may update progress, discoveries, decisions, and outcomes. L5 gates never weaken. L6 ledger is append-only.

## 14. Security

SECURITY.md is binding. No production data in development. No secret in code, logs, prompts, memory, artifacts, or support bundles. Models and agents have intelligence, not authority. Use least privilege and fail closed.

## 15. Definition of done

Node done requires milestones, verify sentinel, expected-files audit, NODE_DONE, and green tag. Run done requires ALL_DONE, fresh verify, production readiness, all required live-fire, signed release, exact manual deploy command, and RUN_COMPLETE.

## 16. Final response

Report nodes completed, changed versus expected files, commands and observed sentinels, criterion status, decisions, assumptions, risks, provider and hardware certification status, ship-gate status, release tag, and confirmation that production was not deployed.
