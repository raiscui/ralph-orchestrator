//! # ralph-proto
//!
//! Shared types, error definitions, and traits for the Ralph Orchestrator framework.
//!
//! This crate provides the foundational abstractions used across all Ralph crates,
//! including:
//! - Event and `EventBus` types for pub/sub messaging
//! - Hat definitions for agent personas
//! - Topic matching for event routing
//! - Common error types

mod error;
mod event;
mod event_bus;
mod gate;
mod hat;
mod routing;
mod topic;
mod ux_event;

pub use error::{Error, Result};
pub use event::{Event, WorkspaceStrategy};
pub use event_bus::EventBus;
pub use gate::{
    GateKind, GateRequest, GateResolve, GateResolvedBy, GateTimeout, TOPIC_GATE_REQUEST,
    TOPIC_GATE_RESOLVE, TOPIC_GATE_TIMEOUT,
};
pub use hat::{Hat, HatId, HatInstanceId, HatInstanceState};
pub use routing::{
    AudienceOverride, AudienceSelector, Delivery, MissingInstancePolicy, QueueDecisionRecord,
    QueueSelection, TOPIC_DISPATCH_DECISION, TopicContract,
};
pub use topic::Topic;
pub use ux_event::{
    FrameCapture, TerminalColorMode, TerminalResize, TerminalWrite, TuiFrame, UxEvent,
};
