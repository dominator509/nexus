#!/usr/bin/env sh
# GraphLock V2 shared helpers. Sourced by node-status-v2.sh, node-close-v2.sh,
# graph-next-v2.sh. Generation-2 closure authority: never trusts NODE_DONE.

# Resolve repository root (arg 1 overrides, else git top-level).
v2_root() {
  if [ -n "${1:-}" ] && [ -d "$1" ]; then
    printf '%s' "$1"
  else
    git rev-parse --show-toplevel 2>/dev/null || printf '%s' "$PWD"
  fi
}

# canonical_digest FILE -> sha256 over canonical JSON (sorted keys, no whitespace)
v2_canonical_digest() {
  python3 - "$1" <<'PY'
import json, hashlib, sys
def canon(obj):
    if isinstance(obj, dict):
        return "{" + ",".join(json.dumps(k, ensure_ascii=False) + ":" + canon(obj[k]) for k in sorted(obj)) + "}"
    if isinstance(obj, list):
        return "[" + ",".join(canon(x) for x in obj) + "]"
    if isinstance(obj, str):
        return json.dumps(obj, ensure_ascii=False)
    if obj is True: return "true"
    if obj is False: return "false"
    if obj is None: return "null"
    return str(obj)
data = json.load(open(sys.argv[1], encoding="utf-8"))
if "attestation_digest" in data:
    data = {k: v for k, v in data.items() if k != "attestation_digest"}
print("sha256:" + hashlib.sha256(canon(data).encode("utf-8")).hexdigest())
PY
}

# digest_of FILE -> sha256 of raw bytes
v2_file_digest() {
  python3 - "$1" <<'PY'
import hashlib, sys
print("sha256:" + hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest())
PY
}

# sorted_list_digest FILE -> sha256 over sorted, trimmed, non-comment lines
v2_list_digest() {
  python3 - "$1" <<'PY'
import hashlib, sys
lines = []
for raw in open(sys.argv[1], encoding="utf-8"):
    s = raw.strip()
    if not s or s.startswith("#"):
        continue
    lines.append(s)
print("sha256:" + hashlib.sha256("\n".join(sorted(lines)).encode("utf-8")).hexdigest())
PY
}
