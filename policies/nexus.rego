# Nexus contextual policy (EP-008 M4) - OPA bundle.
#
# Canonical PolicyInput mapping (see infra/opa/src/mapping.rs):
#   input.tenant_id             tenant id string
#   input.principal_id          principal id string
#   input.principal_type        HUMAN|SERVICE|AGENT|DEVICE|SYSTEM
#   input.capability            QUERY|COMMAND|WORKFLOW|STREAM|ADMINISTRATIVE
#   input.risk                  R0..R4
#   input.strength              NONE|SINGLE_FACTOR|MULTI_FACTOR|STEP_UP
#   input.device_trust          UNVERIFIED|LOCAL|VERIFIED
#   input.device_state          ENABLED|DISABLED|REVOKED (optional)
#   input.object_type           task|memory|household|...
#   input.object_id             canonical id
#   input.sensitivity           PUBLIC|HOUSEHOLD|PERSONAL|SECRET (optional)
#   input.context.location      HOME|WORK|PUBLIC|REMOTE (optional)
#   input.context.network_trust UNTRUSTED|GUEST|TRUSTED (optional)
#   input.context.maintenance   boolean (optional)
#   input.context.emergency     boolean (optional)
#
# Responsibility boundary: OPA evaluates CONTEXTUAL policy only.
# Relationship truth is OpenFGA; risk calculation is nexus-policy;
# approval, approval-digest binding, capability issuance, and action
# execution are separate deterministic layers. OPA never generates
# human approval and never issues grants.
#
# The deterministic gateway (M2) constructs PolicyInput with
# capability=COMMAND, risk=R0, strength=SINGLE_FACTOR,
# device_trust=UNVERIFIED, object_type="action" for its policy stage;
# the policy below is written against that real input shape (verified
# live against the pinned OPA 1.16.2 container).

package nexus

import rego.v1

# ---- Default deny: undefined = deny (directive D, L) ---------------------

default allow := false
default decision := "deny"

decision := "allow" if allow

# Canonical tenant that owns this policy bundle.
canonical_tenant := "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"

# ---- Explicit deny conditions -------------------------------------------

# Tenant mismatch is always denied (fail closed).
deny_tenant_mismatch if input.tenant_id != canonical_tenant

# Disabled/revoked device context is denied when the fact is modeled.
deny_disabled_device if input.device_state in {"DISABLED", "REVOKED"}

# Insufficient authentication strength for the requested capability.
# The gateway's policy stage sends SINGLE_FACTOR; ADMINISTRATIVE
# requires STEP_UP, WORKFLOW requires MULTI_FACTOR.
required_strength := {"QUERY": "SINGLE_FACTOR", "STREAM": "SINGLE_FACTOR",
                      "COMMAND": "SINGLE_FACTOR", "WORKFLOW": "MULTI_FACTOR",
                      "ADMINISTRATIVE": "STEP_UP"}

deny_weak_auth if {
    required_strength[input.capability] != input.strength
    not emergency_allowed
}

# Disallowed context: untrusted network away from home is denied.
deny_untrusted_context if {
    input.context.network_trust == "UNTRUSTED"
    input.context.location != "HOME"
    not emergency_allowed
}

# Sensitive resource requires trusted network context.
deny_sensitive_context if {
    input.sensitivity in {"PERSONAL", "SECRET"}
    input.context.network_trust != "TRUSTED"
    not emergency_allowed
}

# ---- Explicitly modeled exception ----------------------------------------

# Emergency/maintenance exception: ONLY modeled for a verified device in
# the canonical tenant during an explicit emergency flag. It bypasses
# the contextual denies above but never the tenant check.
emergency_allowed if {
    input.context.emergency == true
    input.device_trust == "VERIFIED"
    input.tenant_id == canonical_tenant
}

# ---- Main allow rule -----------------------------------------------------

allow if {
    input.capability in {"QUERY", "COMMAND", "WORKFLOW", "STREAM", "ADMINISTRATIVE"}
    input.risk in {"R0", "R1", "R2", "R3", "R4"}
    input.strength in {"NONE", "SINGLE_FACTOR", "MULTI_FACTOR", "STEP_UP"}
    not deny_tenant_mismatch
    not deny_disabled_device
    not deny_weak_auth
    not deny_untrusted_context
    not deny_sensitive_context
}

# Policy version exposed for audit (safe fingerprint only).
policy_version := "nexus-policy-v1"
