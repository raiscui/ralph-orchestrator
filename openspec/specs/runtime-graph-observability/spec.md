# runtime-graph-observability

## Purpose
Defines Ralph's runtime graph observability requirements:
- keep static hat topology graphs separate from in-run runtime relationship graphs
- expose V1 live runtime topology, lifecycle, workflow, delivery, and reply relations
- define a normalized graph model before binding data to Rerun rendering
- require V2 replay graphs to use durable delivery and lifecycle evidence

## Requirements
### Requirement: The system MUST keep static hat topology graphs and runtime relationship graphs as separate products
The system MUST treat pre-run hat topology visualization and in-run runtime relationship visualization as two different capabilities.

The system MUST preserve the existing `ralph hats graph` responsibility for static topology inspection.

The system MUST NOT redefine the Rerun-based runtime graph as a replacement for static topology diagrams.

#### Scenario: User inspects configured hats before starting a run
- **WHEN** a user wants to inspect subscriptions, publishes, and topology before starting Ralph
- **THEN** the system MUST continue to provide a static topology graph view
- **THEN** the runtime graph capability MUST NOT be required for that inspection

#### Scenario: User inspects a live parallel run
- **WHEN** a user wants to understand which instances were created, which are running, and how messages are flowing during a run
- **THEN** the system MUST provide a runtime graph capability distinct from the static topology graph

### Requirement: V1 live runtime graph MUST expose runtime topology, lifecycle, and workflow relations using existing live observability surfaces
The system MUST support a V1 live runtime graph that visualizes parallel runtime relationships using current live observability signals before introducing new durable delivery records.

V1 MUST cover at least:

- runtime topology nodes such as supervisor and hat instances
- lifecycle state changes for instances
- workflow topic progression
- best-effort reply and queue-decision relations

V1 MAY leave some delivery edges approximate when the current durable model does not provide complete recipient information.

#### Scenario: User watches a live parallel session
- **WHEN** a parallel run is active
- **THEN** the live graph MUST be able to show which instances currently exist
- **THEN** it MUST be able to show whether each visible instance is created, idle, running, done, or failed

#### Scenario: User inspects a workflow bottleneck
- **WHEN** a workflow stalls in the middle of a parallel run
- **THEN** the live graph MUST be able to show the current workflow topic chain and the participating instances
- **THEN** it MAY mark delivery edges as best-effort if the current runtime does not persist complete recipients

### Requirement: Runtime graph data MUST use a normalized node and edge model
The system MUST define a normalized graph data model before binding the visualization to a specific Rerun layout or UI surface.

The normalized node model MUST support at least:

- supervisor nodes
- hat instance nodes
- workflow topic nodes
- optional resource or lane nodes used by runtime coordination

The normalized edge model MUST support at least:

- creates or spawns
- delivers
- replies_to
- publishes
- freezes
- cancels
- shuts_down
- uses_lane

#### Scenario: Same runtime fact rendered in more than one view
- **WHEN** one runtime fact such as `ParallelSupervisor` creating `experiment_runner#3` needs to appear in more than one graph view
- **THEN** the system MUST be able to represent that fact through a stable normalized edge type

#### Scenario: Layout changes but protocol truth remains stable
- **WHEN** the graph layout changes because of force-based rendering or filtering
- **THEN** the underlying node and edge identities MUST remain stable and auditable

### Requirement: V2 durable replay graph MUST be backed by durable delivery and lifecycle evidence
The system MUST define a V2 replay graph that reconstructs runtime relationships from durable artifacts rather than only from live observers.

V2 MUST require durable evidence for at least:

- final delivery recipients such as `target_instance` or fanout recipients
- dynamic instance creation lineage
- lifecycle control edges such as freeze, cancel, and shutdown

The system MUST NOT claim a replay graph is complete if those durable relationships are still missing.

#### Scenario: User replays a finished run after the process exits
- **WHEN** a run has already finished and no live observers remain
- **THEN** the replay graph MUST be reconstructed from durable artifacts
- **THEN** the result MUST still expose the major runtime relationships recorded during that run

#### Scenario: Durable data is incomplete
- **WHEN** a finished run lacks complete recipient or lifecycle control records
- **THEN** the system MUST treat the replay graph as incomplete or approximate
- **THEN** it MUST NOT present that replay graph as a full-fidelity reconstruction
