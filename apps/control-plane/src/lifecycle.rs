//! EP-044 runtime lifecycle (ADR-019 `RuntimeLifecycle`).
//!
//! Graceful startup/shutdown contract: bind once, serve, stop on
//! signal, never leak processes.

use crate::error::{RuntimeError, RuntimeErrorCode};
use crate::vocabulary::RuntimeState;

/// Runtime lifecycle error (fail closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLifecycleError(pub String);

impl std::fmt::Display for RuntimeLifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "runtime lifecycle: {}", self.0)
    }
}

impl std::error::Error for RuntimeLifecycleError {}

impl From<RuntimeLifecycleError> for RuntimeError {
    fn from(value: RuntimeLifecycleError) -> Self {
        RuntimeError::new(RuntimeErrorCode::Conflict, value.0, None)
    }
}

/// Deterministic lifecycle state machine.
///
/// Transitions: Starting -> Ready -> Stopping -> Stopped. Any invalid
/// transition is rejected (fail closed). The runtime never returns to
/// Ready after Stopping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLifecycle {
    state: RuntimeState,
}

impl Default for RuntimeLifecycle {
    fn default() -> Self {
        Self {
            state: RuntimeState::Starting,
        }
    }
}

impl RuntimeLifecycle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> RuntimeState {
        self.state
    }

    /// Mark the runtime ready (Starting -> Ready).
    pub fn mark_ready(&mut self) -> Result<(), RuntimeLifecycleError> {
        if self.state != RuntimeState::Starting {
            return Err(RuntimeLifecycleError(format!(
                "cannot mark ready from {:?}",
                self.state
            )));
        }
        self.state = RuntimeState::Ready;
        Ok(())
    }

    /// Begin graceful shutdown (Ready -> Stopping; Starting is also
    /// allowed so startup failure can stop cleanly).
    pub fn begin_shutdown(&mut self) -> Result<(), RuntimeLifecycleError> {
        match self.state {
            RuntimeState::Ready | RuntimeState::Starting | RuntimeState::Degraded => {
                self.state = RuntimeState::Stopping;
                Ok(())
            }
            RuntimeState::Stopping | RuntimeState::Stopped => Err(RuntimeLifecycleError(
                "runtime is already stopping or stopped".into(),
            )),
        }
    }

    /// Finish shutdown (Stopping -> Stopped).
    pub fn finish_shutdown(&mut self) -> Result<(), RuntimeLifecycleError> {
        if self.state != RuntimeState::Stopping {
            return Err(RuntimeLifecycleError(format!(
                "cannot finish shutdown from {:?}",
                self.state
            )));
        }
        self.state = RuntimeState::Stopped;
        Ok(())
    }

    /// True when the runtime is serving (Ready).
    pub fn is_ready(&self) -> bool {
        self.state == RuntimeState::Ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep044_unit_lifecycle_full_transition() {
        let mut lc = RuntimeLifecycle::new();
        assert_eq!(lc.state(), RuntimeState::Starting);
        lc.mark_ready().unwrap();
        assert!(lc.is_ready());
        lc.begin_shutdown().unwrap();
        assert_eq!(lc.state(), RuntimeState::Stopping);
        lc.finish_shutdown().unwrap();
        assert_eq!(lc.state(), RuntimeState::Stopped);
        assert!(!lc.is_ready());
    }

    #[test]
    fn ep044_unit_lifecycle_rejects_double_ready() {
        let mut lc = RuntimeLifecycle::new();
        lc.mark_ready().unwrap();
        let err = lc.mark_ready().unwrap_err();
        assert!(err.0.contains("cannot mark ready"));
    }

    #[test]
    fn ep044_unit_lifecycle_rejects_shutdown_after_stopped() {
        let mut lc = RuntimeLifecycle::new();
        lc.mark_ready().unwrap();
        lc.begin_shutdown().unwrap();
        lc.finish_shutdown().unwrap();
        let err = lc.begin_shutdown().unwrap_err();
        assert!(err.0.contains("already stopping"));
    }
}
