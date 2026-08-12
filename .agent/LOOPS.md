# BOUNDED EXECUTION LOOPS

## Run loop

Run `sh scripts/graph-next.sh`. Dispatch exactly as GRAPH.md specifies. The finite graph ends in ALL_DONE or a terminal BLOCKED report.

## Node loop

Lease one node. Execute milestones strictly in order. Every milestone uses the verify-fix ladder, commits, and appends evidence. After the last milestone, run node verify and expected-file audit, append NODE_DONE, create the green tag, and release.

## Milestone verify-fix ladder

Normalize the first error line into a signature by removing volatile timestamps, addresses, absolute temporary paths, and counts. Append ATTEMPT_FAIL and SIG. A new signature returns to rung 1, but each milestone has an absolute attempt cap.

1. First same signature: read full output, form one hypothesis, make the smallest targeted fix, and rerun the narrowest command.
2. Second: stop patching and isolate with one diagnostic or one focused test. Confirm or reject the hypothesis before editing.
3. Third: record failed hypotheses, take the milestone's declared real fallback, and append FALLBACK_TAKEN.
4. Fallback exhaustion or absolute cap: rollback to the prior checkpoint and try the fallback once from clean state.
5. Continued failure: append NODE_BLOCKED and the complete report. Never loop again and never fake a pass.

The same fix may not be applied twice for the same signature.

## Readiness loops

A service starts in the background, records its process or container identity, and is probed at most thirty times with two seconds between attempts unless the ExecPlan states a tighter budget. Exhaustion becomes `READINESS_TIMEOUT_<service>` and enters the ladder. Teardown is mandatory.

## Watchdogs

- Three identical commands with identical output force a rung climb.
- Ten actions without a ledger append require an immediate HEARTBEAT.
- After every milestone, compare changed paths with the milestone CHANGE list and `.agent/expected-files/<node>.txt`. Revert unapproved paths unless a Decision Log entry predates retention.
- A milestone exceeding its stated budget enters rung 3 with `BUDGET_EXCEEDED`.
- Provider and hardware certification never use compile success as a substitute for live evidence.

## Re-grounding

Before every milestone, re-read the milestone, the node Non-goals, and `sh scripts/ledger.sh tail 15`. Then run `git status --short` and confirm the current lease.

## Blocked report

The Progress section must contain: exact blocker; commands, output, and exit codes; signatures and hypotheses; every rung and diff; rollback evidence; smallest human decision; recommended default; security and data impact; and exact recovery entry point. NODE_BLOCKED details reference that report.

## Non-interactive mandate

Every command runs unattended with CI, terminal-prompt, pager, and package-manager settings from COMMANDS.md. Credentials are gathered only in PREFLIGHT. Long-running processes are backgrounded and bounded. An unexpected interactive prompt is a defect with signature `UNDECLARED_INTERACTIVE_PROMPT`.
