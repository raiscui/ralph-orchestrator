//! # ralph-core
//!
//! Core orchestration functionality for the Ralph Orchestrator framework.
//!
//! This crate provides:
//! - The main orchestration loop for coordinating multiple agents
//! - Configuration loading and management
//! - State management for agent sessions
//! - Message routing between agents
//! - Terminal capture for session recording
//! - Benchmark task definitions and workspace isolation

pub mod agent_guidance_manifest;
mod agents_snapshot;
pub mod capability;
mod cli_capture;
mod config;
pub mod diagnostics;
mod event_logger;
mod event_loop;
mod event_parser;
mod event_reader;
pub mod evidence_index;
mod experience;
mod experience_governance;
mod experience_injection;
pub mod experience_parser;
mod experience_promotion;
mod experience_store;
mod hat_registry;
mod hatless_ralph;
mod instructions;
mod memory;
pub mod memory_parser;
mod memory_store;
mod parallel;
mod prompt_overlay;
mod session_player;
mod session_recorder;
pub mod state_operations;
mod summary_writer;
pub mod task;
pub mod task_definition;
pub mod task_store;
pub mod testing;
mod text;
pub mod utils;
pub mod workspace;

pub use agents_snapshot::{AgentInstanceSnapshot, AgentLastInput, AgentsSnapshot};
pub use capability::{
    CapabilityChoice, CapabilityFailedRecord, CapabilityInvocationMode, CapabilityInvocationRecord,
    CapabilityKind, CapabilityMetadata, CapabilityParentArtifactPaths,
    CapabilityParentFailedRecord, CapabilityParentResultRecord, CapabilityRequestParseError,
    CapabilityRequestRecord, CapabilityResultRecord, PARENT_CAPABILITY_CATALOG_HEADING,
    RuntimeCapabilityInvoker, TOPIC_CAPABILITY_FAILED, TOPIC_CAPABILITY_INVOKE,
    TOPIC_CAPABILITY_REQUEST, TOPIC_CAPABILITY_RESULT, render_parent_capability_catalog,
};
pub use cli_capture::{CliCapture, CliCapturePair};
pub use config::{
    AllHatPromptConfig, CliConfig, CoreConfig, EventLoopConfig, EventMetadata, GateConfig,
    HatBackend, HatConfig, HatWorkspaceConfig, InjectMode, MemoriesConfig, MemoriesFilter,
    ParallelConfig, PermissionMode, PermissionsConfig, RalphConfig, WorkspaceHooksConfig,
    WorkspaceRuntimeConfig, WorkspaceStrategy,
};
pub use diagnostics::DiagnosticsCollector;
pub use event_logger::{EventHistory, EventLogger, EventRecord};
pub use event_loop::{EventLoop, LoopState, TerminationReason};
pub use event_parser::EventParser;
pub use event_reader::{Event, EventReader, MalformedLine, ParseResult};
pub use evidence_index::{
    EVIDENCE_INDEX_SCHEMA_VERSION, EvidenceArtifactKind, EvidenceIndexEntry, EvidenceIndexError,
    EvidenceIndexReader, EvidenceIndexWriter, EvidenceLookup, EvidenceStatus,
};
pub use experience::{ExperienceConfidence, ExperienceEntry, ExperienceScope, ExperienceStatus};
pub use experience_governance::{
    CanonicalWriterRecord, CanonicalWriterStore, DEFAULT_CANONICAL_WRITER_ID,
    ScopedExperienceInspection, SharedKnowledgeScope, TopicContextFile, TopicContextFileKind,
    TopicContextGroup, WriterGovernanceError, WriterHandoffSummary, WriterOwnerSource,
    detect_topic_groups, detect_unique_topic_group,
};
pub use experience_promotion::{
    DemotionOutcome, ProjectPromotionReason, PromotionDecision, PromotionOutcome,
    RolePromotionDecision, RolePromotionOutcome, RolePromotionSignals, ScopedExperienceError,
    ScopedExperienceService, TopicPromotionSignals, evaluate_role_to_project_promotion,
    evaluate_topic_promotion,
};
pub use experience_store::{
    DEFAULT_PROJECT_EXPERIENCE_PATH, DEFAULT_ROLE_EXPERIENCE_ROOT, MarkdownExperienceStore,
    format_experiences_as_markdown,
};
pub use hat_registry::HatRegistry;
pub use hatless_ralph::{HatInfo, HatTopology, HatlessRalph};
pub use instructions::InstructionBuilder;
pub use memory::{Memory, MemoryType};
pub use memory_store::{
    DEFAULT_MEMORIES_PATH, MarkdownMemoryStore, format_memories_as_markdown, truncate_to_budget,
};
pub use parallel::{
    HatInstanceCommand, HatInstanceEvent, HatInstanceHandle, HatJob, HatJobControl, HatJobExecutor,
    HatJobOutputChunk, HatJobResult, JobBackend, OutputStream, ParallelRunResult,
    ParallelSupervisor, RuntimeDeliveryMode, RuntimeDeliveryObservation, TopicContractStore,
};
pub use session_player::{PlayerConfig, ReplayMode, SessionPlayer, TimestampedRecord};
pub use session_recorder::{Record, SessionRecorder};
pub use state_operations::{
    LifecycleOutcome, RunOutcome, RuntimeStateRecord, StateClearRequest, StateClearResult,
    StateMode, StateOperationError, StateOperationStore, StateReadResult, StateStatus,
    StateWriteRequest, StateWriteResult,
};
pub use summary_writer::SummaryWriter;
pub use task::{Task, TaskStatus};
pub use task_definition::{
    TaskDefinition, TaskDefinitionError, TaskSetup, TaskSuite, Verification,
};
pub use task_store::TaskStore;
pub use text::truncate_with_ellipsis;
pub use workspace::{
    CleanupPolicy, TaskWorkspace, VerificationResult, WorkspaceError, WorkspaceInfo,
    WorkspaceManager,
};
