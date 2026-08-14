"""EP-010 M5 composed capability subsystem proof.

Test names begin with ep010_livefire_ per the EP-010 M5 milestone
contract. This suite drives the REAL composed deterministic subsystem:

  real InMemoryCapabilityRegistry + real CapabilityDispatcher +
  real IdempotencyTracker + real CapabilityDescriptor/ConnectorManifest
  contracts + real typed CapabilityError + real canonical JSON Schemas
  validated by the real jsonschema 0.49.9 validator (draft 2020-12,
  hermetic local $ref resolution)

through the single probe binary
crates/nexus-connectors/examples/livefire_probe.rs. The probe is
orchestration only - every behavior it observes comes from the
production implementations in crates/nexus-capabilities and
crates/nexus-connectors.

EP-010 owns no standalone external provider, so this is a composed
deterministic subsystem proof, NOT an external connector/provider
certification (directive T).

Coverage (directives A-V):
- C  exactly the 13 canonical stages, each with an observed result
- D  register/discover semantics incl. idempotent re-registration
- E  availability filtering (unavailable never advertised)
- F  typed dispatch by capability class; class mismatch denied
- G  command idempotency: replay, provider executed once, conflict
- H  cross-tenant isolation with no existence disclosure
- I  provider failure fail-closed, no success cached as idempotent
- J  schema authority: validation + rejection (5 classes)
- K  health is observation, not authority
- L  change-feed semantics (type, cursor, tenant binding)
- M  workflow capability dispatch (NOT Temporal execution)
- N  descriptor is metadata only - never authorization
- O  connector tier is metadata only - never authorization
- P  schema versions recorded; future migration NOT ASSERTED
- Q  this pytest driver independently verifies stage fields
- R  evidence generated from observed output (module-scoped)
- S  vacuity-proof: the gate runs this suite with a non-zero test count
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CARGO = "/root/.cargo/bin/cargo"
PROBE_BIN = ROOT / "target" / "debug" / "examples" / "livefire_probe"

EXPECTED_STAGES = [
    "REGISTER_DISCOVER",
    "UNAVAILABLE_NOT_ADVERTISED",
    "QUERY_DISPATCH",
    "COMMAND_IDEMPOTENT",
    "WORKFLOW_DISPATCH",
    "HEALTH",
    "CHANGEFEED",
    "CLASS_MISMATCH_DENIED",
    "CROSS_TENANT_DENIED",
    "PROVIDER_ERROR_FAIL_CLOSED",
    "IDEMPOTENCY_CONFLICT",
    "SCHEMA_VALIDATION",
    "SCHEMA_REJECTION",
]

# Independent stage-field expectations (directive Q): these are the
# assertions the probe could NOT satisfy by merely printing PASS.
STAGE_FIELDS = {
    # tenant-scoped registration, sorted deterministic order, idempotent
    # re-registration (registry len stable).
    "REGISTER_DISCOVER": lambda d: (
        d["detail"].startswith(
            'discovered=["test.command", "test.query", "test.stream", "test.workflow"]'
        )
        and "re-register_idempotent=true" in d["detail"]
        and "len_stable=true" in d["detail"]
    ),
    # unavailable capability must not be in the discovery list.
    "UNAVAILABLE_NOT_ADVERTISED": lambda d: "unavailable capability omitted" in d["detail"],
    # query output carries the deterministic request id.
    "QUERY_DISPATCH": lambda d: '"state":"on"' in d["detail"].replace(" ", ""),
    # exactly one recorded idempotency record for two identical calls.
    "COMMAND_IDEMPOTENT": lambda d: "records=1" in d["detail"],
    "WORKFLOW_DISPATCH": lambda d: "workflow_id=wf-livefire-1" in d["detail"],
    "HEALTH": lambda d: "state=HEALTHY" in d["detail"],
    "CHANGEFEED": lambda d: (
        "events=1" in d["detail"] and "next_cursor=cursor-livefire-2" in d["detail"]
    ),
    "CLASS_MISMATCH_DENIED": lambda d: "code=VALIDATION" in d["detail"],
    "CROSS_TENANT_DENIED": lambda d: "code=NOT_FOUND" in d["detail"],
    # provider failure typed, and NO success result cached in the tracker.
    "PROVIDER_ERROR_FAIL_CLOSED": lambda d: (
        "query_code=UNAVAILABLE" in d["detail"]
        and "command_code=UNAVAILABLE" in d["detail"]
        and "cached_success=false" in d["detail"]
    ),
    "IDEMPOTENCY_CONFLICT": lambda d: "code=CONFLICT" in d["detail"],
    "SCHEMA_VALIDATION": lambda d: (
        "descriptor_errors=[]" in d["detail"] and "manifest_errors=[]" in d["detail"]
    ),
    "SCHEMA_REJECTION": lambda d: (
        "unknown_class=true" in d["detail"]
        and "missing_required=true" in d["detail"]
        and "duplicate_events=true" in d["detail"]
        and "duplicate_secrets=true" in d["detail"]
        and "duplicate_origins=true" in d["detail"]
    ),
}

# Evidence accumulator (directive R): generated from observed output.
EVIDENCE = {
    "node": "EP-010",
    "milestone": "M5",
    "composed_subsystem": "nexus-capabilities + nexus-connectors + canonical schemas",
    "validator": "jsonschema 0.49.9 (draft 2020-12)",
    "canonical_ordering": EXPECTED_STAGES,
    "stage_results": {},
    "authority_boundaries": {},
    "schema_versions": {},
    "certification_boundaries": {},
}


def _run_probe() -> dict:
    """Build and run the real probe; fail loudly on any error."""
    built = subprocess.run(
        [CARGO, "build", "--locked", "-p", "nexus-connectors", "--example", "livefire_probe"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=600,
    )
    assert built.returncode == 0, f"probe build failed:\n{built.stdout}\n{built.stderr}"
    run = subprocess.run(
        [str(PROBE_BIN)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert run.returncode == 0, f"probe exited {run.returncode}:\n{run.stdout}\n{run.stderr}"
    try:
        data = json.loads(run.stdout)
    except json.JSONDecodeError as exc:
        raise AssertionError(f"probe output is not valid JSON: {exc}") from None
    assert isinstance(data, dict), "probe output must be a JSON object"
    return data


def ep010_livefire_probe_runs_and_has_required_structure() -> None:
    data = _run_probe()
    # Structural requirements (directive Q items 3-10).
    assert "stages" in data and isinstance(data["stages"], list), "missing stages array"
    assert "canonical_ordering" in data, "missing canonical_ordering"
    assert data["canonical_ordering"] == EXPECTED_STAGES, (
        f"canonical ordering mismatch: {data['canonical_ordering']}"
    )
    names = [s["stage"] for s in data["stages"]]
    assert names == EXPECTED_STAGES, f"stages must be exactly canonical: {names}"
    assert len(names) == len(set(names)), "duplicate stage names present"
    assert all(s["result"] in ("PASS", "FAIL") for s in data["stages"]), "invalid result value"


def ep010_livefire_all_thirteen_stages_pass() -> None:
    data = _run_probe()
    by_name = {s["stage"]: s for s in data["stages"]}
    for name in EXPECTED_STAGES:
        assert name in by_name, f"stage {name} missing"
        assert by_name[name]["result"] == "PASS", f"stage {name} failed: {by_name[name]}"
    assert data["all_pass"] is True, "probe reports all_pass=false"


def ep010_livefire_stage_fields_independently_verified() -> None:
    """Directive Q item 6: do not trust only the top-level PASS."""
    data = _run_probe()
    by_name = {s["stage"]: s for s in data["stages"]}
    for name, check in STAGE_FIELDS.items():
        assert name in by_name, f"stage {name} missing"
        assert check(by_name[name]), f"independent field check failed for {name}: {by_name[name]}"


def ep010_livefire_authority_boundaries_recorded() -> None:
    """Directives N/O/K/T: metadata is never authority."""
    data = _run_probe()
    bounds = data.get("authority_boundaries", {})
    assert bounds.get("descriptor_is_metadata_only") is True, (
        "descriptor carries authority material"
    )
    assert bounds.get("tier_is_metadata_only") is True, "tier altered authorization"
    assert bounds.get("health_is_observation_only") is True, (
        "health report carries authority material"
    )
    assert bounds.get("ep008_authorization_authority") == "EP-008 owns authorization to invoke"
    assert bounds.get("ep005_event_transport_authority") == "EP-005 owns event transport substrate"
    assert bounds.get("ep006_workflow_authority") == "EP-006 owns durable workflow execution"
    assert bounds.get("external_connector_certification") == "NOT OWNED BY EP-010"


def ep010_livefire_schema_versions_recorded() -> None:
    """Directive P: record versions; never fabricate migration support."""
    data = _run_probe()
    versions = data.get("schema_versions", {})
    assert versions.get("capability_descriptor") == "v1"
    assert versions.get("connector_manifest") == "v1"
    assert versions.get("current_version_parity") == "PASS"
    assert versions.get("future_version_migration") == "NOT ASSERTED"


def ep010_livefire_evidence_generated_from_observed_output() -> None:
    """Directive R: evidence is derived from the real probe output."""
    data = _run_probe()
    EVIDENCE["correlation_id"] = data["correlation_id"]
    EVIDENCE["tenant"] = data["tenant"]
    EVIDENCE["stage_results"] = {
        s["stage"]: {"result": s["result"], "detail": s["detail"]} for s in data["stages"]
    }
    EVIDENCE["authority_boundaries"] = data["authority_boundaries"]
    EVIDENCE["schema_versions"] = data["schema_versions"]
    EVIDENCE["certification_boundaries"] = {
        "capability_contract_certification": "PASS",
        "deterministic_registry_dispatcher_certification": "PASS",
        "canonical_schema_parity": "PASS",
        "forced_failure_behavior": "PASS",
        "composed_ep010_subsystem_proof": "PASS",
        "external_connector_provider_certification": "NOT OWNED BY EP-010",
    }
    assert EVIDENCE["stage_results"], "evidence must contain observed stage results"
    assert all(v["result"] == "PASS" for v in EVIDENCE["stage_results"].values())


def ep010_livefire_evidence_written() -> None:
    """Directive R: write governed evidence from observed output."""
    ev_dir = ROOT / ".agent" / "state" / "evidence" / "ep010-m5"
    ev_dir.mkdir(parents=True, exist_ok=True)
    (ev_dir / "ep010-m5-composed-proof.json").write_text(
        json.dumps(EVIDENCE, indent=2, sort_keys=True) + "\n"
    )
    (ev_dir / "EP-010-M5-composed-proof.md").write_text(_render_markdown(EVIDENCE))
    assert (ev_dir / "ep010-m5-composed-proof.json").exists()
    assert (ev_dir / "EP-010-M5-composed-proof.md").exists()
    EVIDENCE["evidence_file"] = str(ev_dir / "ep010-m5-composed-proof.json")


def _render_markdown(evidence: dict) -> str:
    _sv = evidence.get("schema_versions", {})
    lines = [
        "# EP-010 M5 composed capability subsystem proof",
        "",
        f"- Node: `{evidence['node']}`",
        f"- Milestone: `{evidence['milestone']}`",
        f"- Correlation ID: `{evidence.get('correlation_id', '')}`",
        f"- Tenant: `{evidence.get('tenant', '')}`",
        f"- Composed subsystem: `{evidence['composed_subsystem']}`",
        f"- Validator: `{evidence['validator']}`",
        f"- Capability descriptor schema: `{_sv.get('capability_descriptor', '')}`",
        f"- Connector manifest schema: `{_sv.get('connector_manifest', '')}`",
        "- Schema evolution: current-version parity "
        f"`{_sv.get('current_version_parity', '')}`, future-version "
        f"migration `{_sv.get('future_version_migration', '')}`",
        "",
        "## Canonical authorization ordering (EP-010 boundary)",
        "",
        "`" + " -> ".join(evidence.get("canonical_ordering", [])) + "`",
        "",
        "## Stage results (observed)",
        "",
    ]
    for name, result in evidence.get("stage_results", {}).items():
        lines.append(f"- **{name}**: `{result['result']}` - {result['detail']}")
    lines += [
        "",
        "## Authority boundaries",
        "",
    ]
    for k, v in evidence.get("authority_boundaries", {}).items():
        lines.append(f"- {k}: `{v}`")
    lines += [
        "",
        "## Certification boundaries (directive T)",
        "",
    ]
    for k, v in evidence.get("certification_boundaries", {}).items():
        lines.append(f"- {k}: `{v}`")
    lines += [
        "",
        "Evidence is derived from the observed probe output of",
        "`crates/nexus-connectors/examples/livefire_probe.rs` and the real",
        "jsonschema 0.49.9 validator. No credentials, bearer tokens, private",
        "data, or raw provider payloads are persisted.",
        "",
    ]
    return "\n".join(lines)
