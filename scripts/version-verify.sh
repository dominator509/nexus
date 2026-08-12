#!/usr/bin/env sh
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never
fail() { echo "version verify: FAIL - $1" >&2; exit 1; }
[ -f .tool-versions ] || fail "missing .tool-versions"

# Compare installed tool versions against the lock where the tool is present.
# A missing tool on a development host is not a failure (the devcontainer is
# the canonical fallback); a present tool that does not match the lock IS.
python3 - <<'PY' || fail "toolchain comparison"
import re, shutil, subprocess, sys
from pathlib import Path

pins = {}
for line in Path(".tool-versions").read_text().splitlines():
    line = line.strip()
    if not line or line.startswith("#"):
        continue
    parts = line.split()
    if len(parts) == 2:
        pins[parts[0]] = parts[1]

# tool -> (binary, command to print version, regex to extract)
probes = {
    "rust": ("rustc", ["rustc", "--version"], r"rustc (\d+\.\d+\.\d+)"),
    "python": ("python3", ["python3", "--version"], r"Python (\d+\.\d+\.\d+)"),
    "uv": ("uv", ["uv", "--version"], r"uv (\d+\.\d+\.\d+)"),
    "node": ("node", ["node", "--version"], r"v(\d+\.\d+\.\d+)"),
    "pnpm": ("pnpm", ["pnpm", "--version"], r"(\d+\.\d+\.\d+)"),
    "flutter": ("flutter", ["flutter", "--version"], r"Flutter (\d+\.\d+\.\d+)"),
    "opentofu": ("tofu", ["tofu", "version"], r"(\d+\.\d+\.\d+)"),
}
errors = []
for tool, expected in pins.items():
    probe = probes.get(tool)
    if not probe:
        continue
    binary = probe[0]
    if not shutil.which(binary):
        continue  # absent tool: devcontainer fallback is canonical
    try:
        out = subprocess.run(probe[1], capture_output=True, text=True, timeout=20)
        text = (out.stdout + out.stderr)
        m = re.search(probe[2], text)
        actual = m.group(1) if m else "unknown"
        # allow lock's trailing .0 to match tool's shorter version
        norm_expected = expected[:-2] if expected.endswith(".0") else expected
        if actual != expected and actual != norm_expected:
            errors.append(f"{tool}: installed {actual}, lock requires {expected}")
    except Exception as exc:  # noqa: BLE001
        errors.append(f"{tool}: probe failed ({exc})")
if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
print("version verify: installed tools match .tool-versions where present")
PY
echo "version verify: ok"
