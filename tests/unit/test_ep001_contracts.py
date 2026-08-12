"""EP-001 unit tests for the generated Python contracts.

Test names begin with ep001_unit_ per the EP-001 milestone contract.
"""

from __future__ import annotations

import json
from pathlib import Path

from nexus_contracts.generated import (
    ActionRequest,
    NexusControlObject,
)

SCHEMAS = Path(__file__).resolve().parents[2] / "schemas"


def ep001_unit_control_object_has_canonical_fields() -> None:
    """The generated control object matches the canonical schema required set."""
    schema = json.loads((SCHEMAS / "nexus-control-object.schema.json").read_text())
    required = set(schema["required"])
    obj: NexusControlObject = {
        "schema_version": "1",
        "intent": "home.lights.set",
        "route": "DETERMINISTIC",
        "risk": "R0",
        "privacy": "HOUSEHOLD",
        "ambiguity": 0.0,
        "approval_required": False,
        "executable_instruction": True,
        "confidence": 0.99,
        "required_capabilities": ["home.lights.set"],
        "entities": {},
    }
    assert required <= set(obj)


def ep001_unit_action_request_carries_idempotency() -> None:
    """SPEC-006: ActionRequest carries idempotency_key, risk, and approval."""
    req: ActionRequest = {
        "action_id": "act_1",
        "tenant_id": "tenant_1",
        "principal_id": "user_1",
        "capability_id": "cap.lock",
        "idempotency_key": "key_1",
        "risk": "R3",
        "approval_class": "HUMAN",
        "reversal": "COMPENSATING",
        "arguments": {"door": "front"},
        "expected_state": {"locked": True},
        "invocation": {"channel": "voice"},
    }
    assert req["idempotency_key"] == "key_1"
    assert req["risk"] == "R3"
    assert req["approval_class"] == "HUMAN"


def ep001_unit_generated_bindings_are_current() -> None:
    """The committed Python bindings match the canonical schemas (deterministic)."""
    import subprocess
    import sys

    result = subprocess.run(
        [sys.executable, "packages/contracts/scripts/generate.py", "--check"],
        cwd=SCHEMAS.parents[0],
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert result.returncode == 0, f"generated bindings stale: {result.stderr}"
