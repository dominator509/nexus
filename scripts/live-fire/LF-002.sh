#!/usr/bin/env sh
# LF-002 restore-existing-nexus live-fire (EP-037 M5).
#
# Restore encrypted state onto a fresh deployment and prove identities,
# policies, memories, skills, and connectors reattach. The REAL journey
# is the EP-037 M5 gate: encrypted AES-256-GCM state written through
# the production local ArtifactStore, restored onto a genuinely fresh
# deployment root, and reattached through production readback surfaces
# (Principal, RelationshipAuthorizer decision, MemoryRecord validate,
# JsonFileSkillRegistryStore load, capability registry resolve).
# Evidence is current-run bound
# (.agent/state/evidence/LF-002-ep037-m5.json - stale evidence never
# satisfies). The historical phantom runner delegation is gone.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

sh scripts/ep037-m5-tests.sh

echo "LF-002: ok"
