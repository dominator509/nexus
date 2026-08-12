# HOW TO USE THIS BLUEPRINT PACK

## 1. Materialize

The preferred artifact is `Nexus-6Layer-Blueprint.zip`; extract it into an empty repository root. The transcript pack can also be materialized by saving it as `BLUEPRINT_PACK.md`, saving the following script as `unpack.sh`, and running `sh unpack.sh BLUEPRINT_PACK.md`.

```sh
#!/usr/bin/env sh
# 6LAYER pack splitter: materializes files from a pack transcript.
set -eu
pack="${1:-BLUEPRINT_PACK.md}"
[ -f "$pack" ] || { echo "unpack: missing $pack" >&2; exit 1; }
awk '
  /^=== FILE: /{
    path=substr($0, 11)
    sub(/ ===$/, "", path)
    cmd="mkdir -p \"$(dirname \"" path "\")\""
    system(cmd)
    printf "" > path
    out=1
    next
  }
  /^=== END FILE ===$/{ out=0; close(path); next }
  out { print >> path }
' "$pack"
echo "unpack: ok"
```

## 2. Bootstrap

1. Initialize Git if this is a new repository.
2. Run `git add -A && git commit -m "[6LAYER] bootstrap Nexus blueprint pack"`.
3. Read PREFLIGHT.md and select the initial release profile.
4. Copy `.env.example` to `.env`, generate required local secret material, and fill required values.
5. Run `sh scripts/install.sh` to prepare bootstrap dependencies.
6. Run `sh scripts/preflight.sh` until it prints `preflight: ok`.

## 3. Launch the graph

Give any coding agent the complete contents of `.agent/prompts/run-graph.md`.

- Claude Code: invoke Claude Code in its current non-interactive mode with the prompt file contents.
- Codex CLI: invoke Codex from the repository root with approvals disabled only inside the allowed workspace sandbox and pass the prompt file contents.
- Hermes, OpenClaw, Cursor, Cline, Gemini CLI, or another terminal agent: paste the same prompt as the task.

The instruction text is identical. Only the runner-specific non-interactive flag differs.

## 4. Observe without introducing hidden state

Use `tail -f .agent/state/LEDGER.md`, `sh scripts/graph-next.sh`, and `git log --oneline --decorate` to observe progress. Do not make chat history the source of truth. A new agent can resume from Git, the ledger, the active ExecPlan, and the same run-graph prompt.

## 5. Relay or parallel operation

Only one node may hold a lease. An agent that stops appends LEASE_RELEASE and commits. Another agent runs the boot sequence and resumes. A stale lease may be taken over only under GRAPH.md's ninety-minute rule.

## 6. If the graph blocks

When the scheduler prints `BLOCKED EP-NNN`, read the complete report in that ExecPlan Progress section. Make only the named human decision, append the decision to the ledger and plan, reset according to the node recovery section, and relaunch the graph.

## 7. Surgical modes

Use `.agent/prompts/execute-active-execplan.md`, `continue-execplan.md`, `debug-validation-failure.md`, and `final-review.md` for bounded maintenance. Never implement directly from ROADMAP.md.

## 8. Ship decision

The system is shippable only when the ledger contains RUN_COMPLETE and EP-043 observed both `verify: ok` and `production readiness: ok` with all required live-fire and certification evidence. Because auto-deploy is not authorized, the final human action is the exact manual deployment command produced by the signed release manifest.
