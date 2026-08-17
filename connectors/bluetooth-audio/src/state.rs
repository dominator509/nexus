//! Pure connector state machine (SPEC-012 behavior 6, 8).
//!
//! Deterministic transition rules behind the connector: duplicate
//! requests conflict, failed work rolls back to DISCONNECTED, and
//! cancellation never leaves a partial side effect.

use nexus_audio::{AudioError, AudioErrorCode, BluetoothState};

/// Pure transition rules for the Bluetooth endpoint lifecycle.
pub struct ConnectorStateMachine;

impl ConnectorStateMachine {
    /// Begin a connect. Only DISCONNECTED (or unknown) may enter
    /// CONNECTING; any in-flight or established state conflicts
    /// (duplicate request, idempotency contract).
    pub fn begin_connect(current: Option<BluetoothState>) -> Result<BluetoothState, AudioError> {
        match current {
            None | Some(BluetoothState::Disconnected) => Ok(BluetoothState::Connecting),
            Some(BluetoothState::Connecting) => Err(AudioError::new(
                AudioErrorCode::Conflict,
                "duplicate connect while already connecting",
                None,
                None,
            )),
            Some(BluetoothState::Connected) => Err(AudioError::new(
                AudioErrorCode::Conflict,
                "duplicate connect while already connected",
                None,
                None,
            )),
            Some(BluetoothState::Reconnecting) => Err(AudioError::new(
                AudioErrorCode::Conflict,
                "duplicate connect while reconnecting",
                None,
                None,
            )),
        }
    }

    /// Cancel an in-flight transition. Only transitional states may be
    /// cancelled; the result is always DISCONNECTED (no partial side
    /// effect).
    pub fn cancel(current: BluetoothState) -> Result<BluetoothState, AudioError> {
        match current {
            BluetoothState::Connecting | BluetoothState::Reconnecting => {
                Ok(BluetoothState::Disconnected)
            }
            BluetoothState::Connected => Err(AudioError::new(
                AudioErrorCode::Conflict,
                "cannot cancel a connected device; disconnect first",
                None,
                None,
            )),
            BluetoothState::Disconnected => Err(AudioError::new(
                AudioErrorCode::Conflict,
                "cannot cancel a disconnected device",
                None,
                None,
            )),
        }
    }

    /// Roll a failed operation back to DISCONNECTED unconditionally.
    /// This is the compensation path: a failed connect never leaves a
    /// device half-connected.
    pub fn rollback(_current: BluetoothState) -> BluetoothState {
        BluetoothState::Disconnected
    }

    /// Complete a connect after real transport success. Only
    /// transitional states may complete.
    pub fn complete_connect(current: BluetoothState) -> Result<BluetoothState, AudioError> {
        match current {
            BluetoothState::Connecting | BluetoothState::Reconnecting => {
                Ok(BluetoothState::Connected)
            }
            other => Err(AudioError::new(
                AudioErrorCode::Conflict,
                format!("cannot complete connect from {}", other.as_str()),
                None,
                None,
            )),
        }
    }
}
