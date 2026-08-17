//! EP-022 M4 forced-failure suite for the Bluetooth audio connector.
//!
//! Every failure below is a REAL mechanism:
//! - the real system bus (/run/dbus/system_bus_socket) and the real
//!   org.bluez NameHasNoOwner absence on this host;
//! - real Unix sockets that are unreachable, silent, garbage-speaking,
//!   or auth-rejecting (controlled peers, the sanctioned
//!   "corrupt a controlled message" mechanism);
//! - the pure connector state machine (duplicate, cancellation,
//!   rollback).
//!
//! The canary test proves the D-Bus client is live: if it were a stub
//! that always reported "absent", GetNameOwner("org.freedesktop.DBus")
//! against the real bus would fail and the suite would go red.

use std::os::unix::net::UnixListener;
use std::sync::Arc;
use std::time::Duration;

use nexus_audio::{AudioErrorCode, BluetoothDeviceRef, BluetoothEndpointProvider, BluetoothState};
use nexus_bluetooth_audio::audit::IncidentRecorder;
use nexus_bluetooth_audio::connector::BluetoothAudioConnector;
use nexus_bluetooth_audio::dbus::DbusClient;
use nexus_bluetooth_audio::policy::DenyByDefaultPolicy;
use nexus_bluetooth_audio::probe::{BlueZPresence, BlueZProbe};
use nexus_bluetooth_audio::state::ConnectorStateMachine;

const SYSTEM_BUS: &str = "unix:path=/run/dbus/system_bus_socket";
const SHORT: Duration = Duration::from_secs(2);
const TINY: Duration = Duration::from_millis(400);

fn device(name: &str) -> BluetoothDeviceRef {
    BluetoothDeviceRef::new(name).expect("valid test device ref")
}

fn allowlisted() -> Arc<DenyByDefaultPolicy> {
    Arc::new(
        DenyByDefaultPolicy::new()
            .with_allowed([device("AA:BB:CC:DD:EE:FF"), device("11:22:33:44:55:66")]),
    )
}

fn temp_socket_path(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("nexus-ep022-m4-{label}-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir.join("bus.sock")
}

#[test]
fn ep022_failure_canary_probe_is_live_on_real_bus() {
    // The bus itself must have an owner. This proves the D-Bus client
    // performed real authenticated wire traffic on the real system bus.
    let mut client = DbusClient::connect(SYSTEM_BUS, SHORT).expect("connect to real system bus");
    let owner = client
        .get_name_owner("org.freedesktop.DBus")
        .expect("canary owner lookup");
    assert!(!owner.is_empty(), "canary owner must not be empty");
    assert_eq!(owner, "org.freedesktop.DBus");
}

#[test]
fn ep022_failure_bluez_absent_on_real_system_bus() {
    // The real forced-failure substrate: org.bluez has no owner on the
    // real system bus of this host.
    let probe = BlueZProbe::with_address(SYSTEM_BUS, SHORT);
    match probe.probe().expect("real bus probe") {
        BlueZPresence::Absent => {}
        BlueZPresence::Present => panic!(
            "test environment changed: org.bluez is now present on the system bus; \
             the forced-failure substrate is gone"
        ),
    }
}

#[test]
fn ep022_failure_connector_connect_fails_closed_bluez_absent() {
    let connector =
        BluetoothAudioConnector::new(BlueZProbe::with_address(SYSTEM_BUS, SHORT), allowlisted());
    let device = device("AA:BB:CC:DD:EE:FF");
    let error = connector.connect(&device).expect_err("must fail closed");
    assert_eq!(error.code, AudioErrorCode::Unavailable);
    assert!(
        error.message.contains("org.bluez"),
        "error must name the real mechanism: {}",
        error.message
    );
    assert!(
        error.correlation_id.is_some(),
        "error must carry an incident correlation id"
    );
    // Rollback: no partial side effect, no fabricated state.
    assert_eq!(
        connector.state(&device).expect("state readable"),
        BluetoothState::Disconnected
    );
    let metrics = connector.metrics().snapshot();
    assert!(metrics.connect_attempts >= 1);
    assert!(metrics.connect_failures >= 1);
    assert!(metrics.probe_failures >= 1);
    assert!(!connector.audit().is_empty());
}

#[test]
fn ep022_failure_system_bus_unreachable() {
    // A real connect to a nonexistent socket path fails closed.
    let path = temp_socket_path("unreachable");
    let _ = std::fs::remove_file(&path);
    let probe = BlueZProbe::with_address(format!("unix:path={}", path.display()), SHORT);
    let error = probe.probe().expect_err("must fail");
    assert_eq!(error.code, AudioErrorCode::Unavailable);
}

#[test]
fn ep022_failure_bus_peer_silent_times_out() {
    // A real peer that accepts and stalls: the read deadline fires.
    let path = temp_socket_path("silent");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind silent peer");
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            use std::io::Write;
            let _ = stream.write_all(b"");
            std::thread::sleep(Duration::from_millis(3000));
        }
    });
    let probe = BlueZProbe::with_address(format!("unix:path={}", path.display()), TINY);
    let error = probe.probe().expect_err("must time out");
    assert_eq!(error.code, AudioErrorCode::Timeout);
}

#[test]
fn ep022_failure_bus_peer_garbage_rejected() {
    // A real peer speaking non-D-Bus garbage: malformed reply.
    let path = temp_socket_path("garbage");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind garbage peer");
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            use std::io::Write;
            let _ = stream.write_all(b"\x00GARBAGE NOT A BUS\r\n");
            let _ = stream.flush();
            std::thread::sleep(Duration::from_millis(500));
        }
    });
    let probe = BlueZProbe::with_address(format!("unix:path={}", path.display()), SHORT);
    let error = probe.probe().expect_err("must reject garbage");
    assert_eq!(error.code, AudioErrorCode::External);
}

#[test]
fn ep022_failure_bus_peer_auth_rejected() {
    // A real peer that rejects EXTERNAL auth: authorization failure.
    let path = temp_socket_path("authreject");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("bind auth-reject peer");
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            use std::io::Write;
            let _ = stream.write_all(b"REJECTED EXTERNAL\r\n");
            let _ = stream.flush();
            std::thread::sleep(Duration::from_millis(500));
        }
    });
    let probe = BlueZProbe::with_address(format!("unix:path={}", path.display()), SHORT);
    let error = probe.probe().expect_err("must reject auth");
    assert_eq!(error.code, AudioErrorCode::Authorization);
}

#[test]
fn ep022_failure_malformed_device_ref_rejected() {
    let error = BluetoothDeviceRef::new("").expect_err("empty ref must be rejected");
    assert_eq!(error.code, AudioErrorCode::Validation);
    let error =
        BluetoothDeviceRef::new("x".repeat(129)).expect_err("oversized ref must be rejected");
    assert_eq!(error.code, AudioErrorCode::Validation);
    let parsed = BluetoothDeviceRef::new("AA:BB:CC:DD:EE:FF").expect("valid ref accepted");
    assert_eq!(parsed.as_str(), "AA:BB:CC:DD:EE:FF");
}

#[test]
fn ep022_failure_duplicate_connect_conflict() {
    // The state machine rejects a duplicate request while a connect is
    // in flight (idempotency contract, SPEC-012 behavior 8).
    for state in [
        BluetoothState::Connecting,
        BluetoothState::Connected,
        BluetoothState::Reconnecting,
    ] {
        let error = ConnectorStateMachine::begin_connect(Some(state))
            .expect_err("duplicate connect must conflict");
        assert_eq!(error.code, AudioErrorCode::Conflict);
    }
    assert_eq!(
        ConnectorStateMachine::begin_connect(None).expect("fresh connect allowed"),
        BluetoothState::Connecting
    );
    assert_eq!(
        ConnectorStateMachine::begin_connect(Some(BluetoothState::Disconnected))
            .expect("disconnected connect allowed"),
        BluetoothState::Connecting
    );
}

#[test]
fn ep022_failure_cancelled_connect_rolls_back() {
    // Cancellation of an in-flight transition lands DISCONNECTED: no
    // partial side effect.
    assert_eq!(
        ConnectorStateMachine::cancel(BluetoothState::Connecting).expect("cancel connecting"),
        BluetoothState::Disconnected
    );
    assert_eq!(
        ConnectorStateMachine::cancel(BluetoothState::Reconnecting).expect("cancel reconnecting"),
        BluetoothState::Disconnected
    );
    let error = ConnectorStateMachine::cancel(BluetoothState::Connected)
        .expect_err("connected cannot cancel");
    assert_eq!(error.code, AudioErrorCode::Conflict);
    // Connector-level cancellation path (real code path).
    let connector =
        BluetoothAudioConnector::new(BlueZProbe::with_address(SYSTEM_BUS, SHORT), allowlisted());
    connector.cancel_in_flight();
    let device = device("11:22:33:44:55:66");
    let error = connector
        .connect(&device)
        .expect_err("cancelled connect fails");
    assert_eq!(error.code, AudioErrorCode::Conflict);
    assert_eq!(
        connector.state(&device).expect("state readable"),
        BluetoothState::Disconnected
    );
}

#[test]
fn ep022_failure_policy_denied_fails_closed() {
    // Default-deny policy: a non-allowlisted device is a real policy
    // denial; the connector never transitions state.
    let connector = BluetoothAudioConnector::new(
        BlueZProbe::with_address(SYSTEM_BUS, SHORT),
        Arc::new(DenyByDefaultPolicy::new()),
    );
    let device = device("99:88:77:66:55:44");
    let error = connector.connect(&device).expect_err("policy must deny");
    assert_eq!(error.code, AudioErrorCode::Policy);
    assert!(connector.metrics().snapshot().policy_denials >= 1);
    // The device never entered the state map: no fabricated state, no
    // transition attempted (fail closed before any side effect).
    let state_error = connector.state(&device).expect_err("no state may exist");
    assert_eq!(state_error.code, AudioErrorCode::NotFound);
}

#[test]
fn ep022_failure_error_payloads_redacted() {
    // Audit records redact sensitive values; raw audio and credentials
    // never appear.
    let recorder = IncidentRecorder::new();
    recorder.record(
        "UNAVAILABLE",
        Some("corr-1".to_string()),
        Some("AA:BB:CC:DD:EE:FF".to_string()),
        "connect failed secret=supersecret123 token=abc password=hunter2",
    );
    let records = recorder.drain();
    assert_eq!(records.len(), 1);
    assert!(!records[0].message.contains("supersecret123"));
    assert!(!records[0].message.contains("hunter2"));
    assert!(records[0].message.contains("[REDACTED]"));
    assert!(records[0].redacted);
    assert!(records[0].incident_id.starts_with("bt-"));
    assert_eq!(records[0].correlation_id.as_deref(), Some("corr-1"));
    // Structured error dict is redacted by construction (no raw
    // audio/credential keys, typed code preserved).
    let error = nexus_audio::AudioError::unavailable(
        "bluetooth unavailable: org.bluez has no owner on the system bus",
    );
    let dict = error.as_dict();
    assert_eq!(dict["code"], "UNAVAILABLE");
    let serialized = dict.to_string();
    assert!(serialized.contains("org.bluez"));
    assert!(!serialized.to_lowercase().contains("audio frame"));
    assert!(!serialized.to_lowercase().contains("api_key"));
}

#[test]
fn ep022_failure_disconnect_without_transport_fails_closed() {
    let connector =
        BluetoothAudioConnector::new(BlueZProbe::with_address(SYSTEM_BUS, SHORT), allowlisted());
    let ref_a = device("AA:BB:CC:DD:EE:FF");
    // Unknown device: disconnect is NotFound.
    let error = connector.disconnect(&ref_a).expect_err("unknown device");
    assert_eq!(error.code, AudioErrorCode::NotFound);
    // After a failed connect (DISCONNECTED), disconnect is an
    // idempotent no-op - there is nothing to disconnect.
    let _ = connector.connect(&ref_a);
    assert!(connector.disconnect(&ref_a).is_ok());
    // A device never in flight: state is NotFound (no fabricated
    // state).
    let other = device("DE:AD:BE:EF:00:01");
    let error = connector.state(&other).expect_err("unknown state");
    assert_eq!(error.code, AudioErrorCode::NotFound);
}
