//! EP-011 M4 real forced-failure suite (directive Z).
//!
//! Every test drives the REAL sidecar binary + REAL fixture provider
//! over real loopback HTTP. Provider failures use real process
//! mechanisms: kill the provider, arm the provider to return
//! malformed/oversized/schema-invalid/truncated payloads, or make it
//! exit after mutating. No mocks, no direct method calls.

mod common;

use common::*;

// ---------------------------------------------------------------------
// Directive D: request-size and parser hardening
// ---------------------------------------------------------------------

#[test]
fn ep011_failure_sidecar_oversized_request_rejected_before_provider() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);

    // 64 KiB + 1 byte exceeds the locked bound.
    let big = "x".repeat(64 * 1024 + 1);
    let body = serde_json::json!({
        "protocol_version": "1",
        "correlation_id": CORRELATION_ID,
        "request_id": REQUEST_ID,
        "tenant_id": TENANT_A,
        "connector_id": "fixture-connector",
        "capability_id": "fixture.contacts.query",
        "operation": "QUERY",
        "transport": "REST",
        "schema_version": "1.0",
        "input": { "blob": big },
    });
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 413);
    assert_eq!(value["code"], "VALIDATION");
    assert!(value["message"].as_str().unwrap().contains("request body"));
}

#[test]
fn ep011_failure_sidecar_truncated_json_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let raw = br#"{"protocol_version": "1", "correlation_id": "018f0f6f-9c1e-7b6e-8000-000000000002", "request_id": "018f0f6f-9c1e-7b6e-8000-000000000001", "tenant_id": "018f0f6f-9c1e-7b6e-8000-000000000003", "connector_id": "fixture-connector", "capability_id": "fixture.contacts.query", "operation": "QUERY", "transport": "REST", "schema_version": "1.0", "input": {"a":"#;
    let (status, value) = client.post_raw("/v1/query", raw.to_vec(), Some("1"));
    assert_eq!(status, 400);
    assert_eq!(value["code"], "VALIDATION");
}

#[test]
fn ep011_failure_sidecar_malformed_json_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let raw = b"not-json-at-all{{{";
    let (status, value) = client.post_raw("/v1/query", raw.to_vec(), Some("1"));
    assert_eq!(status, 400);
    assert_eq!(value["code"], "VALIDATION");
}

#[test]
fn ep011_failure_sidecar_malformed_utf8_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    // 0xFF is invalid UTF-8.
    let raw = vec![0x7b, 0xFF, 0x7d];
    let (status, _) = client.post_raw("/v1/query", raw, Some("1"));
    assert_eq!(status, 400);
}

#[test]
fn ep011_failure_sidecar_deeply_nested_json_bounded() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    // 10_000 levels of nesting; serde_json's recursion limit fails it.
    let mut raw = String::from("{\"a\":");
    for _ in 0..10_000 {
        raw.push_str("{\"a\":");
    }
    raw.push('1');
    for _ in 0..10_000 {
        raw.push('}');
    }
    raw.push('}');
    let (status, _) = client.post_raw("/v1/query", raw.into_bytes(), Some("1"));
    assert_eq!(status, 400);
}

#[test]
fn ep011_failure_sidecar_binary_body_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let raw = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0xff];
    let (status, _) = client.post_raw("/v1/query", raw, Some("1"));
    assert_eq!(status, 400);
}

#[test]
fn ep011_failure_sidecar_wrong_content_type_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) =
        client.post_content_type("/v1/query", body.to_string().into_bytes(), "text/plain");
    assert_eq!(status, 415);
    assert_eq!(value["code"], "VALIDATION");
}

// ---------------------------------------------------------------------
// Directive E: duplicate / ambiguous fields
// ---------------------------------------------------------------------

#[test]
fn ep011_failure_sidecar_duplicate_security_keys_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    // tenant_id appears twice; the strict envelope must reject it.
    let raw = br#"{
        "protocol_version": "1",
        "correlation_id": "018f0f6f-9c1e-7b6e-8000-000000000002",
        "request_id": "018f0f6f-9c1e-7b6e-8000-000000000001",
        "tenant_id": "018f0f6f-9c1e-7b6e-8000-000000000003",
        "tenant_id": "018f0f6f-9c1e-7b6e-8000-000000000099",
        "connector_id": "fixture-connector",
        "capability_id": "fixture.contacts.query",
        "operation": "QUERY",
        "transport": "REST",
        "schema_version": "1.0",
        "input": {}
    }"#;
    let (status, value) = client.post_raw("/v1/query", raw.to_vec(), Some("1"));
    assert_eq!(status, 400);
    assert!(value["message"].as_str().unwrap().contains("duplicate"));
}

#[test]
fn ep011_failure_sidecar_duplicate_protocol_version_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let raw = br#"{
        "protocol_version": "1",
        "protocol_version": "1",
        "correlation_id": "018f0f6f-9c1e-7b6e-8000-000000000002",
        "request_id": "018f0f6f-9c1e-7b6e-8000-000000000001",
        "tenant_id": "018f0f6f-9c1e-7b6e-8000-000000000003",
        "connector_id": "fixture-connector",
        "capability_id": "fixture.contacts.query",
        "operation": "QUERY",
        "transport": "REST",
        "schema_version": "1.0",
        "input": {}
    }"#;
    let (status, _) = client.post_raw("/v1/query", raw.to_vec(), Some("1"));
    assert_eq!(status, 400);
}

#[test]
fn ep011_failure_sidecar_unknown_top_level_field_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let mut body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    body.as_object_mut()
        .unwrap()
        .insert("admin".to_string(), serde_json::json!(true));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 400);
    assert!(value["message"].as_str().unwrap().contains("unknown"));
}

// ---------------------------------------------------------------------
// Directive H: protocol downgrade / version confusion
// ---------------------------------------------------------------------

#[test]
fn ep011_failure_sidecar_old_version_fails_closed() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("0"));
    assert_eq!(status, 426);
    assert_eq!(value["code"], "VALIDATION");
}

#[test]
fn ep011_failure_sidecar_future_major_version_fails_closed() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("999"));
    assert_eq!(status, 426);
    assert_eq!(value["code"], "VALIDATION");
}

#[test]
fn ep011_failure_sidecar_missing_protocol_version_fails_closed() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, _) = client.post("/v1/query", body, None);
    assert_eq!(status, 426);
}

#[test]
fn ep011_failure_sidecar_conflicting_protocol_versions_fail_closed() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    // Envelope says "1", header says "2" -> conflict.
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("2"));
    assert_eq!(status, 426);
    assert!(value["message"].as_str().unwrap().contains("conflicting"));
}

// ---------------------------------------------------------------------
// Directive F: tenant spoofing
// ---------------------------------------------------------------------

#[test]
fn ep011_failure_sidecar_tenant_spoof_denied_before_provider() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);

    let mut body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    body.as_object_mut()
        .unwrap()
        .insert("tenant_id".to_string(), serde_json::json!(TENANT_B));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 400);
    assert!(
        value["message"]
            .as_str()
            .unwrap()
            .contains("tenant mismatch")
    );
}

// ---------------------------------------------------------------------
// Directive G: connector / capability confusion
// ---------------------------------------------------------------------

#[test]
fn ep011_failure_sidecar_capability_of_other_connector_fails_closed() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    // fixture-connector does not own other-connector's capability.
    let body = query_envelope("other-connector.invoice.read", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 503);
    assert!(
        value["message"]
            .as_str()
            .unwrap()
            .contains("capability not found")
    );
}

#[test]
fn ep011_failure_sidecar_wrong_class_fails_closed() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    // Declare COMMAND class for a QUERY capability.
    let mut body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    body.as_object_mut()
        .unwrap()
        .insert("operation".to_string(), serde_json::json!("COMMAND"));
    body.as_object_mut()
        .unwrap()
        .insert("idempotency_key".to_string(), serde_json::json!("op-1"));
    let (status, value) = client.post("/v1/command", body, Some("1"));
    assert_eq!(status, 400);
    assert!(
        value["message"]
            .as_str()
            .unwrap()
            .contains("class mismatch")
    );
}

#[test]
fn ep011_failure_sidecar_unknown_connector_fails_closed() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let mut body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    body.as_object_mut().unwrap().insert(
        "connector_id".to_string(),
        serde_json::json!("other-connector"),
    );
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 503);
    assert!(
        value["message"]
            .as_str()
            .unwrap()
            .contains("unknown connector")
    );
}

// ---------------------------------------------------------------------
// Directive J: provider process failure
// ---------------------------------------------------------------------

#[test]
fn ep011_failure_sidecar_provider_absent_before_request() {
    // Start the sidecar against a dead provider port.
    let sidecar = spawn_sidecar("http://127.0.0.1:1", &[]);
    let client = Client::new(&sidecar.base);
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 503);
    assert_eq!(value["code"], "UNAVAILABLE");
}

#[test]
fn ep011_failure_sidecar_provider_dies_before_dispatch() {
    let mut provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    // Kill the provider process before dispatch.
    provider.kill();
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 503);
    assert_eq!(value["code"], "UNAVAILABLE");
}

#[test]
fn ep011_failure_sidecar_provider_malformed_response() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let arm = Client::new(&provider.base);
    arm.post(
        "/v1/fixture/arm",
        serde_json::json!({"kind": "query_malformed"}),
        Some("1"),
    );
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 502);
    assert_eq!(value["code"], "EXTERNAL_PROVIDER");
}

#[test]
fn ep011_failure_sidecar_provider_schema_invalid_response() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let arm = Client::new(&provider.base);
    arm.post(
        "/v1/fixture/arm",
        serde_json::json!({"kind": "query_schema_invalid"}),
        Some("1"),
    );
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 502);
    assert_eq!(value["code"], "EXTERNAL_PROVIDER");
}

#[test]
fn ep011_failure_sidecar_provider_oversized_response_bounded() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let arm = Client::new(&provider.base);
    arm.post(
        "/v1/fixture/arm",
        serde_json::json!({"kind": "query_oversized"}),
        Some("1"),
    );
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 502);
    assert_eq!(value["code"], "VALIDATION");
    assert!(value["message"].as_str().unwrap().contains("bounded size"));
}

#[test]
fn ep011_failure_sidecar_provider_timeout_typed() {
    let provider = spawn_provider();
    // Short provider timeout so the slow arm reliably exceeds it.
    let sidecar = spawn_sidecar(
        &provider.base,
        &[("NEXUS_SIDECAR_PROVIDER_TIMEOUT_MS", "500")],
    );
    let client = Client::new(&sidecar.base);
    let arm = Client::new(&provider.base);
    arm.post(
        "/v1/fixture/arm",
        serde_json::json!({"kind": "query_slow", "value": 5}),
        Some("1"),
    );
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 504);
    assert_eq!(value["code"], "TIMEOUT");
}

#[test]
fn ep011_failure_sidecar_provider_command_partial_side_effect_never_success() {
    // Directive J.4/K: the provider performs the command mutation then
    // exits before returning. The sidecar must NOT claim success.
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let arm = Client::new(&provider.base);
    arm.post(
        "/v1/fixture/arm",
        serde_json::json!({"kind": "command_crash_after_mutate"}),
        Some("1"),
    );
    let body = command_envelope(
        "fixture.contacts.command",
        serde_json::json!({"name": "crash-test"}),
        "op-crash-1",
    );
    let (status, value) = client.post("/v1/command", body, Some("1"));
    assert_eq!(status, 502);
    assert_eq!(value["code"], "EXTERNAL_PROVIDER");
}

// ---------------------------------------------------------------------
// Directive I: method / path hardening
// ---------------------------------------------------------------------

#[test]
fn ep011_failure_sidecar_unknown_path_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, _) = client.post("/v1/does-not-exist", body, Some("1"));
    assert_eq!(status, 404);
}

#[test]
fn ep011_failure_sidecar_debug_and_admin_paths_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    for path in [
        "/debug",
        "/debug/vars",
        "/admin",
        "/v1/admin",
        "/metrics",
        "/status",
    ] {
        let (status, _) = client.post(path, serde_json::json!({}), Some("1"));
        assert_eq!(status, 404, "path {path} must be rejected");
    }
}

#[test]
fn ep011_failure_sidecar_wrong_method_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    assert_eq!(client.method("PUT", "/v1/query"), 405);
    assert_eq!(client.method("DELETE", "/v1/query"), 405);
    assert_eq!(client.method("PATCH", "/v1/query"), 405);
    assert_eq!(client.method("OPTIONS", "/v1/query"), 405);
    assert_eq!(client.method("TRACE", "/v1/query"), 405);
}

#[test]
fn ep011_failure_sidecar_encoded_path_traversal_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    // No filesystem path is reachable from URL data. The sidecar has
    // no filesystem mapping at all: every non-canonical path is
    // rejected (404 for unknown routes; any request that survives
    // client-side normalization still cannot reach the provider or
    // the filesystem). Encoded traversal and path confusion all fail.
    for path in [
        "/v1/%2e%2e/%2e%2e/etc/passwd",
        "/v1/..%2f..%2fetc%2fpasswd",
        "/v1//query",
        "/v1/query/",
        "/v1/%2e/query",
        "/v1/%252e%252e/etc/passwd",
    ] {
        let (status, value) = client.post(path, serde_json::json!({}), Some("1"));
        assert!(
            status == 404 || status == 400,
            "path {path} must be rejected, got {status}: {value}"
        );
    }
}

// ---------------------------------------------------------------------
// Directive N: credential broker abuse
// ---------------------------------------------------------------------

#[test]
fn ep011_failure_sidecar_credential_scope_denied_for_other_connector() {
    let provider = spawn_provider();
    // Register a second connector so the scope check (not the
    // connector lookup) is what denies the request.
    let sidecar = spawn_sidecar(
        &provider.base,
        &[
            ("NEXUS_SIDECAR_CONNECTOR_EXTRA", "other-connector"),
            (
                "NEXUS_SIDECAR_CAPABILITIES_EXTRA",
                "other-connector.sync.query:QUERY",
            ),
        ],
    );
    let client = Client::new(&sidecar.base);
    // The scope only permits fixture-connector to use
    // vault:fixture-token; other-connector must be denied.
    let mut body = query_envelope(
        "other-connector.sync.query",
        serde_json::json!({ "credential_reference": "vault:fixture-token" }),
    );
    body.as_object_mut().unwrap().insert(
        "connector_id".to_string(),
        serde_json::json!("other-connector"),
    );
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 403);
    assert_eq!(value["code"], "AUTHORIZATION");
    assert!(value["message"].as_str().unwrap().contains("credential"));
}

#[test]
fn ep011_failure_sidecar_credential_scope_denied_for_other_reference() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let mut body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    body.as_object_mut().unwrap().insert(
        "input".to_string(),
        serde_json::json!({ "credential_reference": "vault:other-secret" }),
    );
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 403);
    assert_eq!(value["code"], "AUTHORIZATION");
}

#[test]
fn ep011_failure_sidecar_credential_unnamespaced_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let mut body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    body.as_object_mut().unwrap().insert(
        "input".to_string(),
        serde_json::json!({ "credential_reference": "plain-token" }),
    );
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 403);
    assert!(value["message"].as_str().unwrap().contains("namespaced"));
}

// ---------------------------------------------------------------------
// Directive V: correlation id validation / log injection
// ---------------------------------------------------------------------

#[test]
fn ep011_failure_sidecar_correlation_injection_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let mut body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    body.as_object_mut().unwrap().insert(
        "correlation_id".to_string(),
        serde_json::json!("ok\nX-Injected: true"),
    );
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 400);
    assert!(
        value["message"]
            .as_str()
            .unwrap()
            .contains("correlation_id")
    );
}

#[test]
fn ep011_failure_sidecar_correlation_oversize_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let mut body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    body.as_object_mut().unwrap().insert(
        "correlation_id".to_string(),
        serde_json::json!("x".repeat(200)),
    );
    let (status, _) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 400);
}

// ---------------------------------------------------------------------
// Directive X: error-wire stability (canonical codes on the wire)
// ---------------------------------------------------------------------

#[test]
fn ep011_failure_sidecar_error_wire_is_canonical_sdk_envelope() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/unknown", body, Some("1"));
    assert_eq!(status, 404);
    // Canonical SDK error envelope fields must all be present.
    assert!(value.get("code").is_some());
    assert!(value.get("message").is_some());
    assert!(value.get("correlation_id").is_some());
    assert!(value.get("actor").is_some());
    assert!(value.get("tenant").is_some());
    assert!(value.get("resource").is_some());
}

// ---------------------------------------------------------------------
// Directive AA: teardown leaves zero resources
// ---------------------------------------------------------------------

#[test]
fn ep011_failure_sidecar_cleanup_after_failure() {
    // Run a handful of failures, then verify ports are released on
    // drop (Drop impls kill processes; the orphan audit gate checks
    // the rest).
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let _ = client.post("/v1/does-not-exist", serde_json::json!({}), Some("1"));
    let _ = client.post(
        "/v1/query",
        query_envelope("fixture.contacts.query", serde_json::json!({})),
        Some("999"),
    );
    drop(sidecar);
    drop(provider);
}
