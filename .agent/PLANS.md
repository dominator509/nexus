# EXECPLAN STANDARD

An ExecPlan is a self-contained implementation document for one graph node. A new agent with no prior conversation must be able to complete it from the plan, AGENTS.md, COMMANDS.md, GRAPH.md, LOOPS.md, accepted specs, schemas, and the ledger.

Every plan begins with NODE-META and contains exactly these sections: Purpose; Scope; Non-goals; Context and Orientation; Files to Read First; Expected Changed Files; Interfaces and Contracts; Milestones; Validation and Acceptance; Idempotence and Recovery; Progress; Surprises and Discoveries; Decision Log; Outcomes and Retrospective.

Each milestone contains GOAL, READ, CHANGE, CONTENT, RUN, EXPECT, EVIDENCE, FALLBACK, and COMMIT. CONTENT is exact enough that no architectural choice remains. If discovery output determines a value, the plan names the command, output field, destination, validation, and safe failure behavior.

Progress checkboxes, discoveries, decisions, and outcomes are the only mutable plan regions. A plan cannot weaken a spec or verification gate. Scope expansion requires a spec update, ADR, graph compatibility review, and ledger entry. A completed plan records actual commands and sentinels, not intended commands.
