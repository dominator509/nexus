//! EP-012 A2A gateway (SPEC-003 required behavior 3).
//!
//! A2A protocol 1.0.1 agent tasks: opaque task lifecycle, streaming
//! status, artifacts, cancellation, and push notifications - mapped to
//! canonical task semantics. Never ordinary data reads; never an
//! authorization mechanism.

#![forbid(unsafe_code)]

pub mod error;
pub mod gateway;
pub mod stream;
pub mod task;

pub use error::{A2AError, A2AErrorCode};
pub use gateway::{A2AGatewayConfig, A2AGatewayImpl, TaskExecutor};
pub use stream::{StreamCursor, StreamEvent, StreamSubscriber};
pub use task::{A2ATaskRecord, A2ATaskStatus, TaskMessage, TaskPriority, TaskStateMachine};
