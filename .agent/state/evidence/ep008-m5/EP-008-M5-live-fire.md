# EP-008 M5 live-fire evidence

- Node: `EP-008`
- Milestone: `M5`

## Providers (real, pinned)

- **openfga**: `1.18.1 sha256:ec73e86c629f7c7b290cde0cf52bcea7c3e0315f30f65386fe4df532f4b83deb`
- **opa**: `1.16.2 sha256:a915d8b59ddb09a9badecd8e061d43cf3111283494c4cf1d38a675bdb4e81a13`
- **policy_bundle**: `policies/nexus.rego (nexus-policy-v1)`

## Canonical authorization ordering

`RELATIONSHIP -> POLICY -> RISK -> APPROVAL -> CAPABILITY -> ALLOW`

## Observed paths

### allow_full_chain
- decision: `ALLOWED`
- stages: `['RELATIONSHIP_PASS', 'POLICY_PASS', 'RISK_R3', 'APPROVAL_PASS', 'CAPABILITY_PASS', 'ALLOWED']`
- risk: `R3`
- policy_version: `nexus-policy-v1`
- receipt_lifecycle: `APPROVED`
- receipt_state: `ISSUED`
- verification_plan: `{'expected': {'state': 'authorization:approved', 'target_id': '0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03'}, 'retries': 3, 'timeout_seconds': 30}`
- relationship_allowed: `True`
- policy_allowed: `True`

### receipt_not_bearer
- copied_receipt_alone: `DENIED NO_CAPABILITY`
- tampered_receipt_alone: `DENIED NO_CAPABILITY`
- stale_receipt_alone: `DENIED NO_CAPABILITY`
- valid_gateway_path: `ALLOWED`

### digest_binding
- changed_element: `action parameter`
- old_approval_new_digest: `DENIED MISSING_APPROVAL`
- new_approval_new_digest: `ALLOWED`

### capability_scope_binding
- wrong_actor: `DENIED NO_CAPABILITY`
- wrong_target: `DENIED NO_CAPABILITY`
- wrong_scope: `DENIED NO_CAPABILITY`
- wrong_tenant: `DENIED NO_CAPABILITY`
- expired_grant: `DENIED NO_CAPABILITY`

### policy_denial_dominates
- decision: `DENIED POLICY`
- stages: `['RELATIONSHIP_PASS', 'POLICY_DENY']`
- risk: `R0`
- receipt_lifecycle: `REJECTED`
- success_receipt: `False`

### relationship_denial_dominates
- decision: `DENIED RELATIONSHIP`
- stages: `['RELATIONSHIP_DENY']`
- restore_after: `ALLOWED`

### r4_model_approval
- model_approval: `DENIED MISSING_APPROVAL (R4)`
- human_approval: `ALLOWED (R4)`

### step_up_required
- below_step_up: `DENIED MISSING_APPROVAL`
- step_up: `ALLOWED`
- approval_does_not_upgrade_strength: `DENIED MISSING_APPROVAL`

### openfga_unavailable
- decision: `ERROR`
- never_allow: `True`
- typed_cause: `OPENFGA: unavailable: cannot reach OpenFGA at http://127.0.0.1:35491/stores/01KZZ30ZKR9W1WJMGT8VF3EF0N/check: Connect error`

### opa_unavailable
- decision: `ERROR`
- never_allow: `True`
- typed_cause: `OPA: unavailable: opa unavailable: cannot reach OPA at http://127.0.0.1:35494/v1/data/nexus/policy_version: Connect error`

### verification_plan
- deterministic_receipt: `True`
- deterministic_plan: `True`
- lifecycle_boundary: `AUTHORIZED != EXECUTED != VERIFIED`
- receipt_lifecycle: `APPROVED`

### no_llm_authority
- relationship_denied: `DENIED despite model ALLOW`
- policy_denied: `DENIED despite model ALLOW`
- missing_approval: `DENIED despite model ALLOW`
- missing_capability: `DENIED despite model ALLOW`

## Boundary

AUTHORIZED != EXECUTED != VERIFIED. EP-008 owns authorization only;
no execution or verification success is claimed. No credentials, bearer
tokens, private data, or raw provider payloads are persisted; evidence
refs are fingerprints and references.
