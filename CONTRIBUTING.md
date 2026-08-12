# CONTRIBUTING

Read AGENTS.md. Work only the node returned by `sh scripts/graph-next.sh`. Use commands from COMMANDS.md. Create no new name, route, table, environment variable, package, provider, schema, capability, or command without repository evidence and a Decision Log entry.

## Standards

Rust is formatted and linted with warnings denied. TypeScript uses strict mode. Python uses typed interfaces, Ruff, and mypy or pyright as locked. Dart follows Flutter analysis. Generated contract code is never manually edited. Comments explain why and cite an `INV-XXX` invariant where relevant.

## Tests

Every behavior change adds or updates unit, contract, integration, failure, E2E, observability, and live-fire evidence as applicable. Required tests cannot be skipped or weakened.

## Documentation

Update specifications before implementations when behavior changes. Update capability, component, version, security, privacy, deployment, operations, and rollback documents in the same milestone.

## Commits

After each milestone: `[EP-XXX][M<k>] <imperative summary>`. Diff must match the milestone CHANGE list. Append ledger evidence and heartbeat.

## Review

Review contract compatibility, security boundary, privacy, authority, failure and rollback, supply chain, cost, local-first behavior, provider replacement, and user experience. A passing build alone is insufficient.
