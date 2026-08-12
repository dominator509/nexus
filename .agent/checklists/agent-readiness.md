# AGENT READINESS

- [ ] Open the active ExecPlan and confirm NODE-META matches GRAPH.md.
- [ ] Run `sh scripts/preflight.sh` and observe `preflight: ok`.
- [ ] Run `sh scripts/graph-next.sh` and confirm the dispatch matches the plan.
- [ ] Confirm Purpose, Non-goals, exact changed files, contracts, milestone fields, fallback, and acceptance are present.
- [ ] Run `sh scripts/expected-files.sh <node>` after the final milestone.
