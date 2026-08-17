#!/usr/bin/env python3
from __future__ import annotations
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def fail(message: str) -> None:
    print(f"blueprint validation: FAIL - {message}", file=sys.stderr)
    raise SystemExit(1)

required = [
    "AGENTS.md", "COMMANDS.md", "PREFLIGHT.md", "PROJECT_BRIEF.md",
    "ARCHITECTURE.md", "COMPONENT_REGISTRY.yaml", "VERSIONS.lock.yaml",
    ".agent/GRAPH.md", ".agent/LOOPS.md", ".agent/state/LEDGER.md",
]
for rel in required:
    if not (ROOT / rel).is_file():
        fail(f"missing {rel}")

graph = (ROOT / ".agent/GRAPH.md").read_text(encoding="utf-8")
inside = False
nodes: list[tuple[str, list[str]]] = []
for line in graph.splitlines():
    if line == "GRAPH-TABLE-BEGIN":
        inside = True
        continue
    if line == "GRAPH-TABLE-END":
        inside = False
        continue
    if inside and line.startswith("NODE "):
        match = re.fullmatch(r"NODE (EP-\d{3}) DEPS ([-A-Z0-9,]+)", line)
        if not match:
            fail(f"invalid graph line: {line}")
        deps = [] if match.group(2) == "-" else match.group(2).split(",")
        nodes.append((match.group(1), deps))
if not nodes:
    fail("graph has no nodes")
seen: set[str] = set()
for node, deps in nodes:
    if node in seen:
        fail(f"duplicate node {node}")
    for dep in deps:
        if dep not in seen:
            fail(f"node {node} dependency {dep} is not earlier in graph")
    seen.add(node)
plans = sorted(p.name[:6] for p in (ROOT / ".agent/execplans").glob("EP-*.md")) if (ROOT / ".agent/execplans").is_dir() else []
if plans and plans != sorted(seen):
    fail("ExecPlan set differs from graph node set")
IGNORE_DIRS = {
    ".git", "node_modules", ".venv", "venv", "target", "dist", "build",
    ".mise", ".cache", "__pycache__", ".pytest_cache", ".mypy_cache",
    ".ruff_cache", ".dart_tool", "coverage",
}

CODE_EXTS = {".py", ".rs", ".ts", ".js"}

# Files that legitimately contain Jinja2 double-braces and are NOT
# unresolved placeholders. The HA fixture configs are REAL template
# entities (light / fan) backed by input_boolean/input_number; Home
# Assistant REQUIRES Jinja2 `{{ }}` syntax for template entities
# (EP-020 M3; EP-024 M3). Narrow allowlist, not a broad gate
# weakening - every other non-code file still fails.
ALLOW_DOUBLE_BRACE = {
    "infra/home-assistant/config/configuration.yaml",
    "connectors/appliances/fixture/config/configuration.yaml",
    "connectors/irrigation/fixture/config/configuration.yaml",
    "connectors/vacuum/fixture/config/configuration.yaml",
}

for path in ROOT.rglob("*"):
    if not path.is_file() or ".git" in path.parts:
        continue
    if any(part in IGNORE_DIRS for part in path.parts):
        continue
    if ".agent/state/evidence" in str(path.relative_to(ROOT)):
        continue
    data = path.read_bytes()
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        continue
    if any(ord(ch) > 127 for ch in text):
        fail(f"non-ASCII text in {path.relative_to(ROOT)}")
    rel = str(path.relative_to(ROOT))
    if (
        "{" * 2 in text
        and path.suffix not in CODE_EXTS
        and rel not in ALLOW_DOUBLE_BRACE
        and path.name not in {"reality-patterns"}
    ):
        fail(f"unresolved double-brace placeholder in {path.relative_to(ROOT)}")
print("blueprint validation: ok")
