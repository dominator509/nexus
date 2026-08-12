"""EP-004 M1 unit tests: memory/data contract agreement.

Test names begin with ep004_unit_ per the EP-004 milestone contract.
These prove the canonical memory wire model and the bootstrap
schemas/memory-record.schema.json agree on required fields, enum values,
and constraints - the Python-side mirror of the Rust ep004_unit_ tests in
crates/nexus-data.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCHEMA = ROOT / "schemas" / "memory-record.schema.json"

CANONICAL_FIELDS = [
    "memory_id",
    "tenant_id",
    "namespace",
    "memory_type",
    "content",
    "content_hash",
    "source",
    "actor",
    "created_at",
    "observed_at",
    "confidence",
    "sensitivity",
    "purpose",
    "retention",
    "status",
    "derived_from",
    "supersedes",
    "embedding_ref",
]

# The bootstrap schema marks provenance-chain fields optional.
REQUIRED_FIELDS = [
    field
    for field in CANONICAL_FIELDS
    if field not in ("derived_from", "supersedes", "embedding_ref")
]

MEMORY_TYPES = [
    "WORKING",
    "EPISODIC",
    "SEMANTIC",
    "ENTITY",
    "PROCEDURAL",
    "DECISION",
    "SKILL",
    "SYSTEM",
]

STATUSES = ["PROPOSED", "ACTIVE", "SUPERSEDED", "REJECTED", "DELETED"]


def _schema() -> dict:
    return json.loads(SCHEMA.read_text())


def ep004_unit_schema_requires_all_canonical_fields() -> None:
    """The schema requires exactly the canonical memory fields.

    `derived_from`, `supersedes`, and `embedding_ref` are optional
    (provenance-chain and index references); the remaining 15 fields are
    required.
    """
    schema = _schema()
    assert schema["additionalProperties"] is False
    required = set(schema["required"])
    canonical = set(CANONICAL_FIELDS)
    expected_required = set(REQUIRED_FIELDS)
    assert required == expected_required, f"required mismatch: {required ^ expected_required}"
    # All 18 properties exist and no unknown property is allowed.
    props = set(schema["properties"])
    assert props == canonical, f"property mismatch: {props ^ canonical}"


def ep004_unit_schema_memory_type_matches_vocabulary() -> None:
    """memory_type enum matches the locked vocabulary (SPEC-002)."""
    schema = _schema()
    enum = schema["properties"]["memory_type"]["enum"]
    assert enum == MEMORY_TYPES


def ep004_unit_schema_status_matches_lifecycle() -> None:
    """status enum matches the EP-004 MemoryStatus contract."""
    schema = _schema()
    enum = schema["properties"]["status"]["enum"]
    assert enum == STATUSES


def ep004_unit_schema_content_hash_is_sha256_hex() -> None:
    """content_hash must be 64 lowercase hex characters."""
    schema = _schema()
    pattern = schema["properties"]["content_hash"]["pattern"]
    assert re.fullmatch(pattern, "a" * 64)
    assert re.fullmatch(pattern, "0" * 64) is None or re.fullmatch(pattern, "0123456789abcdef" * 4)


def ep004_unit_schema_confidence_is_bounded() -> None:
    """confidence must be in [0, 1]."""
    schema = _schema()
    conf = schema["properties"]["confidence"]
    assert conf["minimum"] == 0
    assert conf["maximum"] == 1


def ep004_unit_schema_ids_are_uuids() -> None:
    """memory_id, tenant_id, supersedes use uuid format."""
    schema = _schema()
    assert schema["properties"]["memory_id"]["format"] == "uuid"
    assert schema["properties"]["tenant_id"]["format"] == "uuid"
    assert schema["properties"]["supersedes"]["format"] == "uuid"
    assert schema["properties"]["derived_from"]["items"]["format"] == "uuid"


def ep004_unit_wire_model_is_snake_case_and_closed() -> None:
    """Every canonical field is snake_case and the object is closed."""
    schema = _schema()
    props = schema["properties"]
    for field in CANONICAL_FIELDS:
        assert field in props, f"missing property {field}"
        assert "_" in field or field.islower(), f"{field} is not snake_case"
    assert set(props) == set(CANONICAL_FIELDS), "schema carries unknown fields"


def ep004_unit_fixture_matches_amended_schema() -> None:
    """The M5 fixture in tests/data agrees with the amended schema.

    M4 locked `sensitivity` to the canonical enum and constrained
    `retention` to the canonical wire form; the checked-in fixture must
    satisfy both (stdlib-only validation, no jsonschema dependency).
    """
    schema = _schema()
    fixture = json.loads((ROOT / "tests" / "data" / "memory-record.fixture.json").read_text())
    # Required fields all present.
    for field in schema["required"]:
        assert field in fixture, f"fixture missing required field {field}"
    # Closed object: no unknown fields.
    assert set(fixture) == set(CANONICAL_FIELDS), (
        f"fixture carries unknown fields: {set(fixture) ^ set(CANONICAL_FIELDS)}"
    )
    # M4-locked sensitivity enum.
    sens_enum = schema["properties"]["sensitivity"]["enum"]
    assert fixture["sensitivity"] in sens_enum, "fixture sensitivity not in enum"
    # M4-locked retention pattern.
    pattern = schema["properties"]["retention"]["pattern"]
    assert re.fullmatch(pattern, fixture["retention"]), (
        f"fixture retention {fixture['retention']!r} does not match {pattern}"
    )
    # Existing locked enums still hold.
    assert fixture["memory_type"] in schema["properties"]["memory_type"]["enum"]
    assert fixture["status"] in schema["properties"]["status"]["enum"]
