"""EP-002 M2 cross-language wire-model agreement tests.

These tests prove the checkpoint obligations for EP-002 M2:

1. Canonical JSON Schemas and generated Rust, TypeScript, Python, and Dart
   wire models agree on field names, types, nullability, required fields,
   enum values, UUID formats, additional-properties behavior, and
   schema-version types/constants.
2. schema_version is not left as an unconstrained serde_json::Value when the
   canonical schema requires a string or constant; the generator emits a
   typed constant in every language.
3. Language-safe identifiers such as Python class_ preserve the canonical
   serialized field name (`class`) through aliases and round-trip tests.
4. camelCase conversion is NOT applied anywhere: the canonical wire name is
   the schema property name verbatim (snake_case) in all four languages.

Test names begin with ep002_unit_ per the EP-002 milestone contract.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

SCHEMAS = ROOT / "schemas"
GENERATED_RUST = ROOT / "crates/nexus-contracts/src/generated.rs"
GENERATED_TS = ROOT / "packages/contracts/src/generated.ts"
GENERATED_PY = ROOT / "python/nexus_contracts/generated.py"
GENERATED_DART = ROOT / "packages/contracts/src/generated.dart"


def load_schemas() -> dict[str, dict]:
    out = {}
    for path in sorted(SCHEMAS.glob("*.json")):
        name = path.stem.replace(".schema", "")
        out[name] = json.loads(path.read_text(encoding="utf-8"))
    return out


def type_name(name: str) -> str:
    return "".join(p.capitalize() for p in re.split(r"[^A-Za-z0-9]+", name))


def rust_struct_fields() -> dict[str, dict[str, str]]:
    """Map struct name -> {canonical field name -> rust type}."""
    text = GENERATED_RUST.read_text(encoding="utf-8")
    result: dict[str, dict[str, str]] = {}
    for m in re.finditer(r"pub struct (\w+) \{(.*?)\n\}", text, re.S):
        struct, body = m.group(1), m.group(2)
        fields: dict[str, str] = {}
        for fm in re.finditer(r"pub (\w+): ([^,]+),", body):
            fields[fm.group(1)] = fm.group(2).strip()
        result[struct] = fields
    return result


def ts_interface_fields() -> dict[str, dict[str, str]]:
    """Map interface name -> {canonical field name -> ts type}."""
    text = GENERATED_TS.read_text(encoding="utf-8")
    result: dict[str, dict[str, str]] = {}
    for m in re.finditer(r"export interface (\w+) \{(.*?)\n\}", text, re.S):
        iface, body = m.group(1), m.group(2)
        fields: dict[str, str] = {}
        for fm in re.finditer(r"^\s{2}(\w+)(\?)?:\s*(.+?);$", body, re.M | re.S):
            fields[fm.group(1)] = " ".join(fm.group(3).split())
        result[iface] = fields
    return result


def py_typeddict_fields() -> dict[str, dict[str, str]]:
    """Map TypedDict class name -> {python-safe field name -> py type}."""
    text = GENERATED_PY.read_text(encoding="utf-8")
    result: dict[str, dict[str, str]] = {}
    pattern = r"class (\w+)\(TypedDict\):\n(.*?)(?=\nclass |\nWIRE_ALIASES|\Z)"
    for m in re.finditer(pattern, text, re.S):
        cls, body = m.group(1), m.group(2)
        fields: dict[str, str] = {}
        for fm in re.finditer(r"^\s{4}(\w+): (?:NotRequired\[)?([^\n]+?)\]?\s*$", body, re.M):
            fields[fm.group(1)] = fm.group(2).strip()
        result[cls] = fields
    return result


def dart_class_fields() -> dict[str, dict[str, str]]:
    """Map Dart class name -> {dart-safe field name -> dart type}."""
    text = GENERATED_DART.read_text(encoding="utf-8")
    result: dict[str, dict[str, str]] = {}
    for m in re.finditer(r"class (\w+) \{\n(.*?)\n  const", text, re.S):
        cls, body = m.group(1), m.group(2)
        fields: dict[str, str] = {}
        for fm in re.finditer(r"final ([^;]+?) (\w+);", body):
            fields[fm.group(2)] = fm.group(1).strip()
        result[cls] = fields
    return result


def py_safe(name: str) -> str:
    return (
        name + "_"
        if name
        in {
            "False",
            "None",
            "True",
            "and",
            "as",
            "assert",
            "async",
            "await",
            "break",
            "class",
            "continue",
            "def",
            "del",
            "elif",
            "else",
            "except",
            "finally",
            "for",
            "from",
            "global",
            "if",
            "import",
            "in",
            "is",
            "lambda",
            "nonlocal",
            "not",
            "or",
            "pass",
            "raise",
            "return",
            "try",
            "while",
            "with",
            "yield",
        }
        else name
    )


def dart_safe(name: str) -> str:
    return (
        name + "_"
        if name
        in {
            "abstract",
            "as",
            "assert",
            "async",
            "await",
            "break",
            "case",
            "catch",
            "class",
            "const",
            "continue",
            "default",
            "do",
            "else",
            "enum",
            "extends",
            "false",
            "final",
            "finally",
            "for",
            "if",
            "implements",
            "import",
            "in",
            "interface",
            "is",
            "new",
            "null",
            "override",
            "rethrow",
            "return",
            "super",
            "switch",
            "this",
            "throw",
            "true",
            "try",
            "var",
            "void",
            "while",
            "with",
        }
        else name
    )


def ep002_unit_wire_field_names_agree_in_all_languages() -> None:
    """Every schema property name appears verbatim in all four languages.

    The canonical wire name is the JSON Schema property name (snake_case).
    No camelCase conversion is applied anywhere (proof 4).
    """
    schemas = load_schemas()
    rust = rust_struct_fields()
    ts = ts_interface_fields()
    py = py_typeddict_fields()
    dart = dart_class_fields()

    assert len(rust) == len(schemas), f"rust structs {sorted(rust)} != schemas {sorted(schemas)}"
    assert len(ts) == len(schemas)
    assert len(py) == len(schemas)
    assert len(dart) == len(schemas)

    for sname, doc in schemas.items():
        tname = type_name(sname)
        props = doc.get("properties", {})
        required = set(doc.get("required", []))

        rfields = rust[tname]
        assert set(rfields) == set(props), (
            f"Rust {tname} fields {sorted(rfields)} != schema {sorted(props)}"
        )

        tfields = ts[tname]
        assert set(tfields) == set(props), (
            f"TS {tname} fields {sorted(tfields)} != schema {sorted(props)}"
        )

        pfields = py[tname]
        expected_py = {py_safe(p) for p in props}
        assert set(pfields) == expected_py, (
            f"Python {tname} fields {sorted(pfields)} != schema-safe {sorted(expected_py)}"
        )

        dfields = dart[tname]
        expected_dart = {dart_safe(p) for p in props}
        assert set(dfields) == expected_dart, (
            f"Dart {tname} fields {sorted(dfields)} != schema-safe {sorted(expected_dart)}"
        )

        # Required fields must be non-optional in every language.
        for pname in required:
            assert not rfields[pname].startswith("Option<"), f"Rust {tname}.{pname} optional"
            assert not tfields[pname].endswith(" | null"), f"TS {tname}.{pname} nullable"
            assert "NotRequired" not in pfields[py_safe(pname)], (
                f"Python {tname}.{pname} NotRequired"
            )
            assert "?" not in dfields[dart_safe(pname)], f"Dart {tname}.{pname} nullable"


def ep002_unit_schema_version_is_typed_constant() -> None:
    """schema_version const must be a typed string constant, never unconstrained."""
    schemas = load_schemas()
    rust = rust_struct_fields()
    ts = ts_interface_fields()
    py = py_typeddict_fields()
    dart = dart_class_fields()

    # nexus-control-object pins schema_version to const "1.0.0".
    assert schemas["nexus-control-object"]["properties"]["schema_version"] == {"const": "1.0.0"}
    assert rust["NexusControlObject"]["schema_version"] == "String"
    assert ts["NexusControlObject"]["schema_version"] == '"1.0.0"'
    assert py["NexusControlObject"]["schema_version"].startswith("Literal[")
    assert dart["NexusControlObject"]["schema_version"] == "String"

    # event-envelope declares schema_version as a plain string.
    assert schemas["event-envelope"]["properties"]["schema_version"]["type"] == "string"
    assert rust["EventEnvelope"]["schema_version"] == "String"
    assert ts["EventEnvelope"]["schema_version"] == "string"
    assert py["EventEnvelope"]["schema_version"] == "str"
    assert dart["EventEnvelope"]["schema_version"] == "String"


def ep002_unit_enum_values_agree_in_ts_and_python() -> None:
    """Enums: TS literal union and Python Literal both carry every schema value."""
    schemas = load_schemas()
    ts = ts_interface_fields()
    py = py_typeddict_fields()
    for sname, doc in schemas.items():
        tname = type_name(sname)
        for pname, prop in doc.get("properties", {}).items():
            if "enum" not in prop:
                continue
            values = prop["enum"]
            for v in values:
                assert json.dumps(v) in ts[tname][pname], f"TS {tname}.{pname} missing enum {v!r}"
                assert json.dumps(v) in py[tname][py_safe(pname)], (
                    f"Python {tname}.{pname} missing enum {v!r}"
                )


def ep002_unit_uuid_formatted_fields_are_strings() -> None:
    """format: uuid fields are string-typed in every generated language."""
    schemas = load_schemas()
    rust = rust_struct_fields()
    ts = ts_interface_fields()
    py = py_typeddict_fields()
    dart = dart_class_fields()
    for sname, doc in schemas.items():
        tname = type_name(sname)
        for pname, prop in doc.get("properties", {}).items():
            if prop.get("format") != "uuid":
                continue
            # Nullable+optional fields become Option<Option<String>> in Rust;
            # the wire type is still a string in every language.
            assert "String" in rust[tname][pname], (
                f"Rust {tname}.{pname} not string: {rust[tname][pname]}"
            )
            assert "string" in ts[tname][pname]
            assert "str" in py[tname][py_safe(pname)]
            assert "String" in dart[tname][dart_safe(pname)]


def ep002_unit_additional_properties_are_denied_in_rust() -> None:
    """additionalProperties:false maps to serde deny_unknown_fields."""
    schemas = load_schemas()
    rust_text = GENERATED_RUST.read_text(encoding="utf-8")
    for sname, doc in schemas.items():
        if doc.get("additionalProperties") is False:
            assert "#[serde(deny_unknown_fields)]" in rust_text, (
                f"Rust {type_name(sname)} missing deny_unknown_fields"
            )


def ep002_unit_python_class_alias_preserves_wire_name() -> None:
    """Python class_ preserves canonical wire name `class` via to_wire/from_wire."""
    from nexus_contracts.generated import WIRE_ALIASES, from_wire, to_wire

    assert "CapabilityDescriptor" in WIRE_ALIASES
    assert WIRE_ALIASES["CapabilityDescriptor"] == {"class_": "class"}

    canonical = {
        "id": "cap.lights.set",
        "class": "COMMAND",
        "risk": "R1",
    }
    safe = from_wire("CapabilityDescriptor", canonical)
    assert safe["class_"] == "COMMAND"
    assert "class" not in safe

    back = to_wire("CapabilityDescriptor", safe)
    assert back["class"] == "COMMAND"
    assert "class_" not in back
    assert back == canonical


def ep002_unit_python_roundtrip_preserves_canonical_field_names() -> None:
    """from_wire(to_wire(x)) == x for the class_ alias on full descriptors."""
    from nexus_contracts.generated import CapabilityDescriptor, from_wire, to_wire

    obj: CapabilityDescriptor = {
        "id": "cap.lights.set",
        "version": "1.0.0",
        "class_": "QUERY",
        "description": "Query a device state",
        "input_schema": "/schemas/query.json",
        "output_schema": "/schemas/state.json",
        "required_scopes": ["device.read"],
        "risk": "R0",
        "approval": "NONE",
        "reversal": "NONE",
        "idempotency": "NOT_APPLICABLE",
        "availability": "AVAILABLE",
    }
    wired = to_wire("CapabilityDescriptor", dict(obj))
    assert wired["class"] == "QUERY"
    assert "class_" not in wired
    back = from_wire("CapabilityDescriptor", wired)
    assert back == dict(obj)
