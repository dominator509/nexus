//! Real Bluetooth endpoint provider: probe-gated, policy-checked,
//! fail-closed, audited (SPEC-012 behavior 7).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use nexus_audio::{
    AudioError, AudioErrorCode, BluetoothDeviceRef, BluetoothEndpointProvider, BluetoothState,
};

use crate::audit::{IncidentRecorder, Metrics};
use crate::policy::BluetoothConnectPolicy;
use crate::probe::{BlueZPresence, BlueZProbe};
use crate::state::ConnectorStateMachine;

/// Real `BluetoothEndpointProvider` implementation.
///
/// `connect` is a real three-gate path: policy decision, duplicate
/// detection, then a real system-bus probe for org.bluez. Every
/// failure is typed, audited, counted, and rolls back to DISCONNECTED.
/// The CONNECTED state is never fabricated: it is reachable only after
/// a real certified transport exists (deferred).
pub struct BluetoothAudioConnector {
    probe: BlueZProbe,
    policy: Arc<dyn BluetoothConnectPolicy>,
    states: Mutex<HashMap<BluetoothDeviceRef, BluetoothState>>,
    audit: IncidentRecorder,
    metrics: Metrics,
    cancelled: AtomicBool,
    next_correlation: AtomicU64,
}

impl BluetoothAudioConnector {
    pub fn new(probe: BlueZProbe, policy: Arc<dyn BluetoothConnectPolicy>) -> Self {
        Self {
            probe,
            policy,
            states: Mutex::new(HashMap::new()),
            audit: IncidentRecorder::new(),
            metrics: Metrics::new(),
            cancelled: AtomicBool::new(false),
            next_correlation: AtomicU64::new(1),
        }
    }

    pub fn audit(&self) -> &IncidentRecorder {
        &self.audit
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Cancel any in-flight operation (bounded recovery lever).
    pub fn cancel_in_flight(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn clear_cancellation(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }

    pub fn probe(&self) -> &BlueZProbe {
        &self.probe
    }

    fn next_correlation(&self) -> Box<str> {
        format!(
            "bt-conn-{:x}",
            self.next_correlation.fetch_add(1, Ordering::Relaxed)
        )
        .into()
    }

    fn audit_failure(&self, error: &AudioError, correlation: &str, device: &BluetoothDeviceRef) {
        self.audit.record(
            error.code.as_str(),
            Some(correlation.to_string()),
            Some(device.to_string()),
            &error.message,
        );
    }
}

impl BluetoothEndpointProvider for BluetoothAudioConnector {
    fn connect(&self, device: &BluetoothDeviceRef) -> Result<(), AudioError> {
        self.metrics.incr_connect_attempts();
        if let Err(policy_error) = self.policy.allow_connect(device) {
            self.metrics.incr_policy_denials();
            let correlation = self.next_correlation();
            let mut error = policy_error;
            error.correlation_id = Some(correlation.clone());
            self.audit_failure(&error, &correlation, device);
            return Err(error);
        }
        let mut states = self.states.lock().unwrap();
        let current = states.get(device).copied();
        // Duplicate request detection (idempotency contract).
        let next = ConnectorStateMachine::begin_connect(current).map_err(|mut e| {
            let correlation = self.next_correlation();
            e.correlation_id = Some(correlation.clone());
            self.audit_failure(&e, &correlation, device);
            e
        })?;
        states.insert(device.clone(), next);
        // Cancellation before the real probe: roll back, fail closed.
        if self.cancelled.swap(false, Ordering::SeqCst) {
            states.insert(device.clone(), BluetoothState::Disconnected);
            let correlation = self.next_correlation();
            let error = AudioError::new(
                AudioErrorCode::Conflict,
                "connect cancelled before transport probe",
                Some(correlation.clone()),
                Some(Box::from(device.to_string())),
            );
            self.audit_failure(&error, &correlation, device);
            return Err(error);
        }
        // Real probe of the system bus for org.bluez.
        match self.probe.probe() {
            Ok(BlueZPresence::Present) => {
                // BlueZ is running, but the real Bluetooth audio
                // transport is not certified on this host. Never
                // fabricate CONNECTED; fail closed with the deferred
                // certification named.
                states.insert(device.clone(), BluetoothState::Disconnected);
                self.metrics.incr_connect_failures();
                let correlation = self.next_correlation();
                let error = AudioError::new(
                    AudioErrorCode::Unavailable,
                    "bluez is present but the bluetooth audio transport is not certified on this host (deferred)",
                    Some(correlation.clone()),
                    Some(Box::from(device.to_string())),
                );
                self.audit_failure(&error, &correlation, device);
                Err(error)
            }
            Ok(BlueZPresence::Absent) => {
                // The real forced-failure substrate: org.bluez has no
                // owner on the real system bus.
                states.insert(device.clone(), BluetoothState::Disconnected);
                self.metrics.incr_connect_failures();
                self.metrics.incr_probe_failures();
                let correlation = self.next_correlation();
                let error = AudioError::new(
                    AudioErrorCode::Unavailable,
                    "bluetooth unavailable: org.bluez has no owner on the system bus",
                    Some(correlation.clone()),
                    Some(Box::from(device.to_string())),
                );
                self.audit_failure(&error, &correlation, device);
                Err(error)
            }
            Err(probe_error) => {
                states.insert(device.clone(), BluetoothState::Disconnected);
                self.metrics.incr_connect_failures();
                self.metrics.incr_probe_failures();
                let correlation = self.next_correlation();
                let mut error = probe_error;
                error.correlation_id = Some(correlation.clone());
                self.audit_failure(&error, &correlation, device);
                Err(error)
            }
        }
    }

    fn disconnect(&self, device: &BluetoothDeviceRef) -> Result<(), AudioError> {
        let states = self.states.lock().unwrap();
        match states.get(device).copied() {
            None => Err(AudioError::new(
                AudioErrorCode::NotFound,
                "bluetooth device is not tracked",
                None,
                Some(Box::from(device.to_string())),
            )),
            Some(BluetoothState::Disconnected) => {
                // Idempotent: there is nothing to disconnect.
                Ok(())
            }
            Some(_) => {
                let correlation = self.next_correlation();
                let error = AudioError::new(
                    AudioErrorCode::Unavailable,
                    "cannot disconnect without a real bluetooth transport",
                    Some(correlation.clone()),
                    Some(Box::from(device.to_string())),
                );
                self.audit_failure(&error, &correlation, device);
                Err(error)
            }
        }
    }

    fn state(&self, device: &BluetoothDeviceRef) -> Result<BluetoothState, AudioError> {
        let states = self.states.lock().unwrap();
        states.get(device).copied().ok_or_else(|| {
            AudioError::new(
                AudioErrorCode::NotFound,
                "bluetooth device is not tracked",
                None,
                Some(Box::from(device.to_string())),
            )
        })
    }
}
