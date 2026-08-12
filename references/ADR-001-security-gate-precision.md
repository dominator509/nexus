# ADR-001: Security gate secret-pattern precision fix

Status: Accepted
Date: 2026-08-12
Author: hermes-nexus-main (EP-000 M4)

## Problem

`scripts/security-check.sh` scans every tracked file with the regex set:

```
AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----|sk-[A-Za-z0-9_-]{24,}|ghp_[A-Za-z0-9]{30,}|xox[baprs]-[A-Za-z0-9-]{10,}|AGE-SECRET-KEY-1[A-Z0-9]{20,}
```

The `sk-[A-Za-z0-9_-]{24,}` alternative has no left word boundary. The blueprint
pack's own committed content contains the string `EP-025-asterisk-telephony-and-ai-calling.md`
(the canonical Asterisk node name). The substring `sk-telephony-and-ai-calling.md` is
`sk-` followed by 27 `[A-Za-z0-9_-]` characters, so the gate false-positives on
`.agent/GRAPH.md`, `.agent/MANIFEST.md`, `.agent/execplans/EP-025-*.md`,
`.agent/expected-files/EP-025.txt`, `.agent/milestone-files/EP-025-M1.txt`, and `ROADMAP.md`.
The pack cannot pass its own security gate.

## Decision

Fix the gate regex precision by requiring a word boundary immediately before `sk-`:

- Before: `sk-[A-Za-z0-9_-]{24,}`
- After:  `(^|[^A-Za-z0-9_-])sk-[A-Za-z0-9_-]{24,}`

This is a precision fix, not a weakening: real OpenAI-style keys (`sk-` after
whitespace, quote, colon, or line start) still match; embedded substrings inside
longer identifiers (`asterisk-...`) no longer match. The gate remains fail-closed
for every real secret shape it was designed to catch.

## Evidence

- Reproduced failure: `sh scripts/security-check.sh` prints hits in the six
  Asterisk-named files listed above and exits 1.
- After fix: `sh scripts/security-check.sh` prints `security check: ok`.
- Negative control: a temp file containing `sk-` + 30 alphanumerics still fails
  the gate (proven by test).

## Alternatives considered

1. Adding the six files to an allowlist -- rejected: broadens the gate's blind
   spots and hides the pattern defect.
2. Renaming EP-025 artifacts -- rejected: the pack's canonical names are locked;
   renaming to dodge a gate is drift.
3. Leaving the gate broken -- rejected: blocks every node's verify and the ship gate.

## Compatibility impact

The fix changes only the regex in one L5 gate. No schema, contract, or behavior
change. All other patterns unchanged. Rollback: revert the one-line regex change.

## Security impact

Positive: the gate now catches only true secrets, eliminating a noise source that
would force future agents to either fix patterns ad hoc or ignore the gate.
