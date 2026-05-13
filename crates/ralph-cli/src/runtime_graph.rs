//! Rerun live runtime graph recorder for parallel mode.
//!
//! 说明:
//! - 这是 `rerun-runtime-graphs` change 的 V1 MVP:
//!   - 基于现有 live observers 记录 runtime topology / workflow / delivery
//!   - 输出 `.rrd` artifact,供后续用 `rerun <file>` 打开
//! - 这里刻意把“规范化数据模型”和“Rerun 写入”放在一起,
//!   但中间先经过 `RuntimeGraphState` 的稳定节点/边集合,避免“看到什么就直接画什么”。

use anyhow::{Context, Result};
use ralph_core::{EventHistory, EventRecord, RuntimeDeliveryMode, RuntimeDeliveryObservation};
use ralph_proto::{
    Event, HatInstanceId, HatInstanceState, RuntimeDeliveryRecord, RuntimeLifecycleKind,
    RuntimeLifecycleRecord, TOPIC_DISPATCH_DECISION, TOPIC_REQUESTER_RETURN,
    TOPIC_RUNTIME_DELIVERY, TOPIC_RUNTIME_LIFECYCLE,
};
use rerun::{GraphEdges, GraphNodes, RecordingStream, RecordingStreamBuilder};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;
use tracing::warn;

const APPLICATION_ID: &str = "ralph_runtime_graph";
const TIMELINE_STEP: &str = "runtime_step";
const TIMELINE_ELAPSED: &str = "seconds_since_start";

const PATH_TOPOLOGY_NODES: &str = "runtime_graph/runtime_topology/nodes";
const PATH_TOPOLOGY_CREATES: &str = "runtime_graph/runtime_topology/creates";
const PATH_TOPOLOGY_SPAWNS: &str = "runtime_graph/runtime_topology/spawns";
const PATH_TOPOLOGY_FREEZES: &str = "runtime_graph/runtime_topology/freezes";
const PATH_TOPOLOGY_CANCELS: &str = "runtime_graph/runtime_topology/cancels";
const PATH_TOPOLOGY_SHUTDOWNS: &str = "runtime_graph/runtime_topology/shutdowns";
const PATH_WORKFLOW_NODES: &str = "runtime_graph/workflow/nodes";
const PATH_WORKFLOW_PUBLISHES: &str = "runtime_graph/workflow/publishes";
const PATH_WORKFLOW_DELIVERS: &str = "runtime_graph/workflow/delivers";
const PATH_DELIVERY_NODES: &str = "runtime_graph/delivery/nodes";
const PATH_DELIVERY_DIRECT: &str = "runtime_graph/delivery/direct";
const PATH_DELIVERY_QUEUE: &str = "runtime_graph/delivery/queue";
const PATH_DELIVERY_FANOUT: &str = "runtime_graph/delivery/fanout";
const PATH_DELIVERY_REPLY: &str = "runtime_graph/delivery/reply";

const NODE_SUPERVISOR: &str = "supervisor";

#[derive(Debug, Clone)]
struct RuntimeNode {
    id: String,
    label: String,
    color: [u8; 3],
    radius: f32,
}

#[derive(Debug, Clone)]
struct RuntimeEdge {
    source: String,
    target: String,
}

#[derive(Debug, Clone, Default)]
struct RuntimeGraphSnapshot {
    step: u64,
    topology_nodes: Vec<RuntimeNode>,
    topology_creates: Vec<RuntimeEdge>,
    topology_spawns: Vec<RuntimeEdge>,
    topology_freezes: Vec<RuntimeEdge>,
    topology_cancels: Vec<RuntimeEdge>,
    topology_shutdowns: Vec<RuntimeEdge>,
    workflow_nodes: Vec<RuntimeNode>,
    workflow_publishes: Vec<RuntimeEdge>,
    workflow_delivers: Vec<RuntimeEdge>,
    delivery_nodes: Vec<RuntimeNode>,
    delivery_direct: Vec<RuntimeEdge>,
    delivery_queue: Vec<RuntimeEdge>,
    delivery_fanout: Vec<RuntimeEdge>,
    delivery_reply: Vec<RuntimeEdge>,
}

#[derive(Debug, Default)]
struct RuntimeGraphState {
    step: u64,
    topology_nodes: BTreeMap<String, RuntimeNode>,
    topology_creates: BTreeMap<String, RuntimeEdge>,
    topology_spawns: BTreeMap<String, RuntimeEdge>,
    topology_freezes: BTreeMap<String, RuntimeEdge>,
    topology_cancels: BTreeMap<String, RuntimeEdge>,
    topology_shutdowns: BTreeMap<String, RuntimeEdge>,
    workflow_nodes: BTreeMap<String, RuntimeNode>,
    workflow_publishes: BTreeMap<String, RuntimeEdge>,
    workflow_delivers: BTreeMap<String, RuntimeEdge>,
    delivery_nodes: BTreeMap<String, RuntimeNode>,
    delivery_direct: BTreeMap<String, RuntimeEdge>,
    delivery_queue: BTreeMap<String, RuntimeEdge>,
    delivery_fanout: BTreeMap<String, RuntimeEdge>,
    delivery_reply: BTreeMap<String, RuntimeEdge>,
}

impl RuntimeGraphState {
    fn apply_instance_state(&mut self, instance_id: &HatInstanceId, state: HatInstanceState) {
        self.ensure_supervisor();

        let node = instance_node(instance_id, state);
        self.topology_nodes.insert(node.id.clone(), node.clone());
        self.workflow_nodes
            .entry(node.id.clone())
            .or_insert_with(|| workflow_instance_node(instance_id));
        self.delivery_nodes
            .entry(node.id.clone())
            .or_insert_with(|| delivery_instance_node(instance_id));

        let create_edge_id = format!("creates:{NODE_SUPERVISOR}:{}", instance_id.as_str());
        self.topology_creates
            .entry(create_edge_id.clone())
            .or_insert_with(|| RuntimeEdge {
                source: NODE_SUPERVISOR.to_string(),
                target: instance_id.as_str().to_string(),
            });
    }

    fn apply_event(&mut self, event: &Event) {
        self.ensure_supervisor();

        let topic_node = topic_node(event.topic.as_str());
        self.workflow_nodes
            .insert(topic_node.id.clone(), topic_node.clone());
        self.delivery_nodes
            .insert(topic_node.id.clone(), topic_node.clone());

        let source_node = event
            .source_instance
            .as_ref()
            .map_or_else(|| NODE_SUPERVISOR.to_string(), |id| id.as_str().to_string());

        if let Some(source_instance) = &event.source_instance {
            self.workflow_nodes
                .entry(source_instance.as_str().to_string())
                .or_insert_with(|| workflow_instance_node(source_instance));
            self.delivery_nodes
                .entry(source_instance.as_str().to_string())
                .or_insert_with(|| delivery_instance_node(source_instance));
        }

        let publish_edge_id = format!("publishes:{}:{}", source_node, topic_node.id);
        self.workflow_publishes
            .entry(publish_edge_id.clone())
            .or_insert_with(|| RuntimeEdge {
                source: source_node,
                target: topic_node.id.clone(),
            });

        if let Some(target_instance) = &event.target_instance {
            self.workflow_nodes
                .entry(target_instance.as_str().to_string())
                .or_insert_with(|| workflow_instance_node(target_instance));

            let deliver_edge_id = format!(
                "workflow_delivers:{}:{}:{}",
                event.topic,
                topic_node.id,
                target_instance.as_str()
            );
            self.workflow_delivers
                .entry(deliver_edge_id.clone())
                .or_insert_with(|| RuntimeEdge {
                    source: topic_node.id.clone(),
                    target: target_instance.as_str().to_string(),
                });
        }
    }

    fn apply_event_record(&mut self, record: &EventRecord) {
        self.ensure_supervisor();

        let topic_node = topic_node(record.topic.as_str());
        self.workflow_nodes
            .insert(topic_node.id.clone(), topic_node.clone());
        self.delivery_nodes
            .insert(topic_node.id.clone(), topic_node.clone());

        let source_node = record
            .source_instance
            .as_deref()
            .unwrap_or(NODE_SUPERVISOR)
            .to_string();

        if let Some(source_instance) = &record.source_instance {
            let instance_id = HatInstanceId::new(source_instance.clone());
            self.workflow_nodes
                .entry(source_node.clone())
                .or_insert_with(|| workflow_instance_node(&instance_id));
            self.delivery_nodes
                .entry(source_node.clone())
                .or_insert_with(|| delivery_instance_node(&instance_id));
        }

        let publish_edge_id = format!("publishes:{}:{}", source_node, topic_node.id);
        self.workflow_publishes
            .entry(publish_edge_id.clone())
            .or_insert_with(|| RuntimeEdge {
                source: source_node,
                target: topic_node.id,
            });
    }

    fn apply_delivery(&mut self, observation: &RuntimeDeliveryObservation) {
        self.ensure_supervisor();

        let recipient_id = observation.recipient.as_str().to_string();
        self.delivery_nodes
            .entry(recipient_id.clone())
            .or_insert_with(|| delivery_instance_node(&observation.recipient));

        let topic = topic_node(&observation.topic);
        self.delivery_nodes
            .entry(topic.id.clone())
            .or_insert_with(|| topic.clone());

        let source = observation
            .source_instance
            .as_ref()
            .map_or_else(|| topic.id.clone(), |id| id.as_str().to_string());

        if let Some(source_instance) = &observation.source_instance {
            self.delivery_nodes
                .entry(source_instance.as_str().to_string())
                .or_insert_with(|| delivery_instance_node(source_instance));
        }

        let edge_id = format!(
            "{}:{}:{}:{}",
            observation.mode.as_str(),
            observation.topic,
            source,
            recipient_id
        );
        let edge = RuntimeEdge {
            source,
            target: recipient_id,
        };

        match observation.mode {
            RuntimeDeliveryMode::Direct => {
                self.delivery_direct.entry(edge_id).or_insert(edge);
            }
            RuntimeDeliveryMode::Queue => {
                self.delivery_queue.entry(edge_id).or_insert(edge);
            }
            RuntimeDeliveryMode::Fanout => {
                self.delivery_fanout.entry(edge_id).or_insert(edge);
            }
            RuntimeDeliveryMode::Reply => {
                self.delivery_reply.entry(edge_id).or_insert(edge);
            }
        }
    }

    fn apply_delivery_record(&mut self, record: &RuntimeDeliveryRecord) {
        self.apply_delivery(&RuntimeDeliveryObservation {
            topic: record.topic.clone(),
            source_instance: record.source_instance.clone(),
            recipient: record.recipient.clone(),
            mode: record.mode,
        });
    }

    fn apply_lifecycle_record(&mut self, record: &RuntimeLifecycleRecord) {
        self.ensure_supervisor();

        let state = record.state.unwrap_or(HatInstanceState::Created);
        let node = instance_node(&record.instance_id, state);
        self.topology_nodes.insert(node.id.clone(), node.clone());
        self.workflow_nodes
            .entry(node.id.clone())
            .or_insert_with(|| workflow_instance_node(&record.instance_id));
        self.delivery_nodes
            .entry(node.id.clone())
            .or_insert_with(|| delivery_instance_node(&record.instance_id));

        match record.kind {
            RuntimeLifecycleKind::Create => {
                Self::insert_topology_edge(
                    &mut self.topology_creates,
                    "creates",
                    NODE_SUPERVISOR,
                    record.instance_id.as_str(),
                );
            }
            RuntimeLifecycleKind::Spawn => {
                Self::insert_topology_edge(
                    &mut self.topology_spawns,
                    "spawns",
                    NODE_SUPERVISOR,
                    record.instance_id.as_str(),
                );
            }
            RuntimeLifecycleKind::State => {}
            RuntimeLifecycleKind::Freeze => {
                Self::insert_topology_edge(
                    &mut self.topology_freezes,
                    "freezes",
                    NODE_SUPERVISOR,
                    record.instance_id.as_str(),
                );
            }
            RuntimeLifecycleKind::Cancel => {
                Self::insert_topology_edge(
                    &mut self.topology_cancels,
                    "cancels",
                    NODE_SUPERVISOR,
                    record.instance_id.as_str(),
                );
            }
            RuntimeLifecycleKind::Shutdown => {
                Self::insert_topology_edge(
                    &mut self.topology_shutdowns,
                    "shutdowns",
                    NODE_SUPERVISOR,
                    record.instance_id.as_str(),
                );
            }
        }
    }

    fn insert_topology_edge(
        collection: &mut BTreeMap<String, RuntimeEdge>,
        kind: &str,
        source: &str,
        target: &str,
    ) {
        let edge_id = format!("{kind}:{source}:{target}");
        collection.entry(edge_id).or_insert_with(|| RuntimeEdge {
            source: source.to_string(),
            target: target.to_string(),
        });
    }

    fn snapshot_and_advance(&mut self) -> RuntimeGraphSnapshot {
        self.step = self.step.saturating_add(1);
        RuntimeGraphSnapshot {
            step: self.step,
            topology_nodes: self.topology_nodes.values().cloned().collect(),
            topology_creates: self.topology_creates.values().cloned().collect(),
            topology_spawns: self.topology_spawns.values().cloned().collect(),
            topology_freezes: self.topology_freezes.values().cloned().collect(),
            topology_cancels: self.topology_cancels.values().cloned().collect(),
            topology_shutdowns: self.topology_shutdowns.values().cloned().collect(),
            workflow_nodes: self.workflow_nodes.values().cloned().collect(),
            workflow_publishes: self.workflow_publishes.values().cloned().collect(),
            workflow_delivers: self.workflow_delivers.values().cloned().collect(),
            delivery_nodes: self.delivery_nodes.values().cloned().collect(),
            delivery_direct: self.delivery_direct.values().cloned().collect(),
            delivery_queue: self.delivery_queue.values().cloned().collect(),
            delivery_fanout: self.delivery_fanout.values().cloned().collect(),
            delivery_reply: self.delivery_reply.values().cloned().collect(),
        }
    }

    fn ensure_supervisor(&mut self) {
        let supervisor = supervisor_node();
        self.topology_nodes
            .entry(supervisor.id.clone())
            .or_insert_with(|| supervisor.clone());
        self.workflow_nodes
            .entry(supervisor.id.clone())
            .or_insert_with(|| supervisor.clone());
        self.delivery_nodes
            .entry(supervisor.id.clone())
            .or_insert(supervisor);
    }
}

/// Offline replay graph 的过滤条件。
#[derive(Debug, Clone, Default)]
pub struct RuntimeGraphReplayFilter {
    pub topic: Option<String>,
    pub instance: Option<HatInstanceId>,
}

impl RuntimeGraphReplayFilter {
    fn matches_event_record(&self, record: &EventRecord) -> bool {
        if let Some(topic) = &self.topic
            && record.topic != *topic
        {
            return false;
        }

        if let Some(instance) = &self.instance {
            return record.source_instance.as_deref() == Some(instance.as_str());
        }

        true
    }

    fn matches_delivery_record(&self, record: &RuntimeDeliveryRecord) -> bool {
        if let Some(topic) = &self.topic
            && record.topic != *topic
        {
            return false;
        }

        if let Some(instance) = &self.instance {
            return record.recipient == *instance
                || record.source_instance.as_ref() == Some(instance);
        }

        true
    }

    fn matches_lifecycle_record(&self, record: &RuntimeLifecycleRecord) -> bool {
        if let Some(instance) = &self.instance {
            return record.instance_id == *instance;
        }

        true
    }
}

/// Offline replay graph 的重建结果摘要。
#[derive(Debug, Clone)]
pub struct RuntimeGraphReplayReport {
    pub output_path: PathBuf,
    pub records_read: usize,
    pub workflow_records: usize,
    pub delivery_records: usize,
    pub lifecycle_records: usize,
    pub lifecycle_control_records: usize,
    pub full_fidelity: bool,
}

/// Live runtime graph recorder.
pub struct RuntimeGraphRecorder {
    output_path: PathBuf,
    recording: RecordingStream,
    started_at: Instant,
    state: Mutex<RuntimeGraphState>,
}

impl RuntimeGraphRecorder {
    /// Creates a new `.rrd` recorder.
    pub fn create(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create runtime graph parent directory: {}",
                    parent.display()
                )
            })?;
        }

        let recording = RecordingStreamBuilder::new(APPLICATION_ID)
            .save(&path)
            .with_context(|| {
                format!(
                    "Failed to create runtime graph recording: {}",
                    path.display()
                )
            })?;

        let recorder = Self {
            output_path: path,
            recording,
            started_at: Instant::now(),
            state: Mutex::new(RuntimeGraphState::default()),
        };

        recorder.with_state_update(|state| state.ensure_supervisor());
        Ok(recorder)
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }

    pub fn replay_from_events(
        events_path: impl AsRef<Path>,
        output_path: impl Into<PathBuf>,
        filter: RuntimeGraphReplayFilter,
    ) -> Result<RuntimeGraphReplayReport> {
        let output_path = output_path.into();
        let history = EventHistory::new(events_path.as_ref());
        let records = history.read_all().with_context(|| {
            format!(
                "Failed to read runtime graph replay events: {}",
                events_path.as_ref().display()
            )
        })?;

        let recorder = Self::create(output_path.clone())?;
        let mut report = RuntimeGraphReplayReport {
            output_path,
            records_read: records.len(),
            workflow_records: 0,
            delivery_records: 0,
            lifecycle_records: 0,
            lifecycle_control_records: 0,
            full_fidelity: false,
        };

        for record in &records {
            match record.topic.as_str() {
                TOPIC_RUNTIME_DELIVERY => {
                    let delivery: RuntimeDeliveryRecord = serde_json::from_str(&record.payload)
                        .with_context(|| {
                            format!("Invalid runtime.delivery payload: {}", record.payload)
                        })?;
                    if filter.matches_delivery_record(&delivery) {
                        recorder.with_state_update(|graph| {
                            graph.apply_delivery_record(&delivery);
                        });
                        report.delivery_records += 1;
                    }
                }
                TOPIC_RUNTIME_LIFECYCLE => {
                    let lifecycle: RuntimeLifecycleRecord = serde_json::from_str(&record.payload)
                        .with_context(|| {
                        format!("Invalid runtime.lifecycle payload: {}", record.payload)
                    })?;
                    if filter.matches_lifecycle_record(&lifecycle) {
                        if matches!(
                            lifecycle.kind,
                            RuntimeLifecycleKind::Freeze
                                | RuntimeLifecycleKind::Cancel
                                | RuntimeLifecycleKind::Shutdown
                        ) {
                            report.lifecycle_control_records += 1;
                        }
                        recorder.with_state_update(|graph| {
                            graph.apply_lifecycle_record(&lifecycle);
                        });
                        report.lifecycle_records += 1;
                    }
                }
                TOPIC_DISPATCH_DECISION | TOPIC_REQUESTER_RETURN => {}
                _ => {
                    if filter.matches_event_record(record) {
                        recorder.with_state_update(|graph| {
                            graph.apply_event_record(record);
                        });
                        report.workflow_records += 1;
                    }
                }
            }
        }

        report.full_fidelity = report.delivery_records > 0
            && report.lifecycle_records > 0
            && report.lifecycle_control_records > 0;
        recorder.finish();
        Ok(report)
    }

    pub fn observe_instance_state(&self, instance_id: &HatInstanceId, state: HatInstanceState) {
        self.with_state_update(|graph| graph.apply_instance_state(instance_id, state));
    }

    pub fn observe_event(&self, event: &Event) {
        self.with_state_update(|graph| graph.apply_event(event));
    }

    pub fn observe_delivery(&self, observation: &RuntimeDeliveryObservation) {
        self.with_state_update(|graph| graph.apply_delivery(observation));
    }

    pub fn finish(&self) {
        if let Err(e) = self.recording.flush_blocking() {
            warn!(error = %e, path = %self.output_path.display(), "Failed to flush runtime graph recording");
        }
        self.recording.disconnect();
    }

    fn with_state_update(&self, update: impl FnOnce(&mut RuntimeGraphState)) {
        let snapshot = {
            let mut guard = self
                .state
                .lock()
                .expect("runtime graph recorder mutex poisoned");
            update(&mut guard);
            guard.snapshot_and_advance()
        };

        if let Err(e) = self.log_snapshot(snapshot) {
            warn!(error = %e, path = %self.output_path.display(), "Failed to write runtime graph snapshot");
        }
    }

    fn log_snapshot(&self, snapshot: RuntimeGraphSnapshot) -> Result<()> {
        self.recording.set_time_sequence(
            TIMELINE_STEP,
            i64::try_from(snapshot.step).unwrap_or(i64::MAX),
        );
        self.recording
            .set_duration_secs(TIMELINE_ELAPSED, self.started_at.elapsed().as_secs_f64());

        self.log_nodes(PATH_TOPOLOGY_NODES, &snapshot.topology_nodes)?;
        self.log_edges(PATH_TOPOLOGY_CREATES, &snapshot.topology_creates)?;
        self.log_edges(PATH_TOPOLOGY_SPAWNS, &snapshot.topology_spawns)?;
        self.log_edges(PATH_TOPOLOGY_FREEZES, &snapshot.topology_freezes)?;
        self.log_edges(PATH_TOPOLOGY_CANCELS, &snapshot.topology_cancels)?;
        self.log_edges(PATH_TOPOLOGY_SHUTDOWNS, &snapshot.topology_shutdowns)?;

        self.log_nodes(PATH_WORKFLOW_NODES, &snapshot.workflow_nodes)?;
        self.log_edges(PATH_WORKFLOW_PUBLISHES, &snapshot.workflow_publishes)?;
        self.log_edges(PATH_WORKFLOW_DELIVERS, &snapshot.workflow_delivers)?;

        self.log_nodes(PATH_DELIVERY_NODES, &snapshot.delivery_nodes)?;
        self.log_edges(PATH_DELIVERY_DIRECT, &snapshot.delivery_direct)?;
        self.log_edges(PATH_DELIVERY_QUEUE, &snapshot.delivery_queue)?;
        self.log_edges(PATH_DELIVERY_FANOUT, &snapshot.delivery_fanout)?;
        self.log_edges(PATH_DELIVERY_REPLY, &snapshot.delivery_reply)?;

        Ok(())
    }

    fn log_nodes(&self, entity_path: &str, nodes: &[RuntimeNode]) -> Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }

        let archetype = GraphNodes::new(nodes.iter().map(|node| node.id.clone()))
            .with_labels(nodes.iter().map(|node| node.label.clone()))
            .with_colors(nodes.iter().map(|node| node.color))
            .with_radii(nodes.iter().map(|node| node.radius))
            .with_show_labels(true);

        self.recording
            .log(entity_path, &archetype)
            .with_context(|| format!("Failed to log runtime graph nodes: {entity_path}"))
    }

    fn log_edges(&self, entity_path: &str, edges: &[RuntimeEdge]) -> Result<()> {
        if edges.is_empty() {
            return Ok(());
        }

        let archetype = GraphEdges::new(
            edges
                .iter()
                .map(|edge| (edge.source.clone(), edge.target.clone())),
        )
        .with_directed_edges();

        self.recording
            .log(entity_path, &archetype)
            .with_context(|| format!("Failed to log runtime graph edges: {entity_path}"))
    }
}

fn supervisor_node() -> RuntimeNode {
    RuntimeNode {
        id: NODE_SUPERVISOR.to_string(),
        label: "supervisor".to_string(),
        color: [44, 164, 140],
        radius: 16.0,
    }
}

fn instance_node(instance_id: &HatInstanceId, state: HatInstanceState) -> RuntimeNode {
    RuntimeNode {
        id: instance_id.as_str().to_string(),
        label: format!("{}\n{}", instance_id.as_str(), state.as_str()),
        color: state_color(state),
        radius: 13.0,
    }
}

fn workflow_instance_node(instance_id: &HatInstanceId) -> RuntimeNode {
    RuntimeNode {
        id: instance_id.as_str().to_string(),
        label: instance_id.as_str().to_string(),
        color: [78, 122, 255],
        radius: 11.0,
    }
}

fn delivery_instance_node(instance_id: &HatInstanceId) -> RuntimeNode {
    RuntimeNode {
        id: instance_id.as_str().to_string(),
        label: instance_id.as_str().to_string(),
        color: [78, 122, 255],
        radius: 11.0,
    }
}

fn topic_node(topic: &str) -> RuntimeNode {
    RuntimeNode {
        id: format!("topic::{topic}"),
        label: topic.to_string(),
        color: [255, 170, 66],
        radius: 9.0,
    }
}

fn state_color(state: HatInstanceState) -> [u8; 3] {
    match state {
        HatInstanceState::Created => [100, 116, 139],
        HatInstanceState::Running => [34, 197, 94],
        HatInstanceState::Idle => [59, 130, 246],
        HatInstanceState::Done => [107, 114, 128],
        HatInstanceState::Failed => [239, 68, 68],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_core::{EventLogger, RuntimeDeliveryMode, RuntimeDeliveryObservation};
    use ralph_proto::{Event, RuntimeDeliveryRecord, RuntimeLifecycleKind, RuntimeLifecycleRecord};

    #[test]
    fn instance_state_registers_supervisor_and_create_edge() {
        let mut state = RuntimeGraphState::default();
        let instance_id = HatInstanceId::from("writer#1");

        state.apply_instance_state(&instance_id, HatInstanceState::Created);

        assert!(state.topology_nodes.contains_key(NODE_SUPERVISOR));
        assert!(state.topology_nodes.contains_key("writer#1"));
        assert!(
            state
                .topology_creates
                .contains_key("creates:supervisor:writer#1")
        );
    }

    #[test]
    fn workflow_event_registers_topic_and_publish_edge() {
        let mut state = RuntimeGraphState::default();
        let event = Event::new("build.task", "do work")
            .with_source_instance(HatInstanceId::from("planner#1"))
            .with_target_instance(HatInstanceId::from("builder#1"));

        state.apply_event(&event);

        assert!(state.workflow_nodes.contains_key("topic::build.task"));
        assert!(
            state
                .workflow_publishes
                .contains_key("publishes:planner#1:topic::build.task")
        );
        assert!(
            state
                .workflow_delivers
                .contains_key("workflow_delivers:build.task:topic::build.task:builder#1")
        );
    }

    #[test]
    fn delivery_observation_routes_to_mode_specific_collection() {
        let mut state = RuntimeGraphState::default();
        let observation = RuntimeDeliveryObservation {
            topic: "review.done".to_string(),
            source_instance: Some(HatInstanceId::from("reviewer#1")),
            recipient: HatInstanceId::from("ralph#1"),
            mode: RuntimeDeliveryMode::Reply,
        };

        state.apply_delivery(&observation);

        assert!(
            state
                .delivery_reply
                .contains_key("reply:review.done:reviewer#1:ralph#1")
        );
        assert!(state.delivery_nodes.contains_key("reviewer#1"));
        assert!(state.delivery_nodes.contains_key("ralph#1"));
        assert!(state.delivery_nodes.contains_key("topic::review.done"));
    }

    #[test]
    fn replay_from_events_builds_full_fidelity_graph_from_v2_records() {
        let temp_dir = tempfile::tempdir().unwrap();
        let events_path = temp_dir.path().join("events.jsonl");
        let output_path = temp_dir.path().join("runtime.rrd");

        let mut logger = EventLogger::new(&events_path);
        let event = Event::new("build.task", "do work")
            .with_id("event-1")
            .with_source_instance(HatInstanceId::from("planner#1"));
        logger
            .log_event(0, "planner", &event, None)
            .expect("workflow event should be logged");

        let delivery = RuntimeDeliveryRecord::new(
            Some("event-1".to_string()),
            None,
            "build.task",
            Some(HatInstanceId::from("planner#1")),
            HatInstanceId::from("writer#1"),
            RuntimeDeliveryMode::Queue,
        );
        logger
            .log_runtime_delivery(0, "supervisor", &delivery)
            .expect("delivery record should be logged");

        for lifecycle in [
            RuntimeLifecycleRecord::new(
                HatInstanceId::from("writer#1"),
                RuntimeLifecycleKind::Create,
            )
            .with_state(HatInstanceState::Created),
            RuntimeLifecycleRecord::new(
                HatInstanceId::from("writer#1"),
                RuntimeLifecycleKind::Freeze,
            )
            .with_reason("completion_promise"),
            RuntimeLifecycleRecord::new(
                HatInstanceId::from("writer#1"),
                RuntimeLifecycleKind::Cancel,
            )
            .with_reason("supervisor_shutdown"),
            RuntimeLifecycleRecord::new(
                HatInstanceId::from("writer#1"),
                RuntimeLifecycleKind::Shutdown,
            )
            .with_reason("supervisor_shutdown"),
        ] {
            logger
                .log_runtime_lifecycle(0, "supervisor", &lifecycle)
                .expect("lifecycle record should be logged");
        }

        let report = RuntimeGraphRecorder::replay_from_events(
            &events_path,
            &output_path,
            RuntimeGraphReplayFilter::default(),
        )
        .expect("replay should succeed");

        assert_eq!(report.records_read, 6);
        assert_eq!(report.workflow_records, 1);
        assert_eq!(report.delivery_records, 1);
        assert_eq!(report.lifecycle_records, 4);
        assert_eq!(report.lifecycle_control_records, 3);
        assert!(report.full_fidelity);
        assert!(std::fs::metadata(output_path).unwrap().len() > 0);
    }

    #[test]
    fn replay_from_events_marks_approximate_without_v2_records() {
        let temp_dir = tempfile::tempdir().unwrap();
        let events_path = temp_dir.path().join("events.jsonl");
        let output_path = temp_dir.path().join("runtime.rrd");

        let mut logger = EventLogger::new(&events_path);
        let event = Event::new("build.task", "do work")
            .with_id("event-1")
            .with_source_instance(HatInstanceId::from("planner#1"));
        logger
            .log_event(0, "planner", &event, None)
            .expect("workflow event should be logged");

        let report = RuntimeGraphRecorder::replay_from_events(
            &events_path,
            &output_path,
            RuntimeGraphReplayFilter::default(),
        )
        .expect("replay should succeed");

        assert_eq!(report.records_read, 1);
        assert_eq!(report.workflow_records, 1);
        assert_eq!(report.delivery_records, 0);
        assert_eq!(report.lifecycle_records, 0);
        assert!(!report.full_fidelity);
        assert!(std::fs::metadata(output_path).unwrap().len() > 0);
    }
}
