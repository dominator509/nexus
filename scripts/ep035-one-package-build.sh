#!/usr/bin/env sh
# EP-035 M5 one-package deployment bundle builder (LF-001).
#
# Builds the Nexus Setup one-package deployment bundle from the CURRENT
# source tree: the canonical deployment-profile schema, the onboarding
# DDL migrations, and the built @nexus/onboarding runtime. The bundle is
# the LF-001 one-package artifact: source commit -> production artifact
# -> deterministic SHA-256 identity. The artifact hash depends only on
# file contents (sorted manifest), so the same commit always produces
# the same identity.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

# pnpm resolves from PATH (mise shims on CI and dev); PNPM_BIN may
# override. A hardcoded /root mise path is NOT portable - CI runners
# run as a non-root user with no /root access (EP-043 lesson).
PNPM="${PNPM_BIN:-pnpm}"
repo="$(pwd)"
commit="$(git rev-parse HEAD)"
out="$repo/.agent/state/livefire/ep035-bundle"

rm -rf "$out"
mkdir -p "$out/schemas" "$out/migrations" "$out/runtime"

# 1. Canonical deployment profile schema (the local provider profile
#    contract LF-001 drives the deployment with).
cp "$repo/schemas/deployment-profile.schema.json" "$out/schemas/"

# 2. Onboarding durable DDL migrations (the package carries its own
#    schema; the live-fire applies it from the bundle, not the dev tree).
cp "$repo/packages/onboarding/migrations/001_onboarding.sql" "$out/migrations/"

# 3. The real onboarding runtime built from source.
(cd "$repo/packages/onboarding" && "$PNPM" exec tsc -p tsconfig.build.json)
cp -r "$repo/packages/onboarding/dist/." "$out/runtime/"

# 4. Deterministic manifest with per-file SHA-256 and the artifact hash.
python3 - "$out" "$commit" <<'PY'
import hashlib
import json
import os
import sys

out, commit = sys.argv[1], sys.argv[2]
files = {}
for root, dirs, names in os.walk(out):
    dirs.sort()
    for name in sorted(names):
        if name == "MANIFEST.json":
            continue
        p = os.path.join(root, name)
        rel = os.path.relpath(p, out)
        with open(p, "rb") as fh:
            files[rel] = hashlib.sha256(fh.read()).hexdigest()
canon = "".join(f"{k}:{v}\n" for k, v in sorted(files.items()))
artifact_hash = hashlib.sha256(canon.encode()).hexdigest()
manifest = {
    "artifact": "nexus-setup-one-package",
    "git_commit": commit,
    "files": files,
    "artifact_hash": artifact_hash,
}
with open(os.path.join(out, "MANIFEST.json"), "w") as fh:
    json.dump(manifest, fh, indent=2, sort_keys=True)
    fh.write("\n")
print(f"artifact_hash={artifact_hash}")
print(f"bundle={out}")
PY
