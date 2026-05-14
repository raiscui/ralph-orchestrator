# Test plan: runtime-evidence-index-kernel

## Scope

This test plan covers Phase 1A only: minimal evidence index kernel. It intentionally excludes Phase 1B evidence CLI / doctor UX.

## Test layers

### 1. Schema contract tests

Goal: prove the minimal index entry can represent required evidence links without display-only fields.

Planned assertions:

- `EvidenceIndexEntry` serializes with `schema_version`, `session_id` or `run_id`, `correlation_id`, `artifact_kind`, `artifact_path`, `producer`, `status`.
- Optional `parent_correlation_id` and `child_correlation_id` serialize only when present.
- The schema can represent `success`, `failure`, `missing`, and `unknown` statuses.
- The schema does not contain CLI summary, doctor diagnosis, graph layout, or rendered display fields.

### 2. Writer / reader lookup tests

Goal: prove tests and future runtime code can find artifacts by correlation id.

Planned assertions:

- Writer records multiple entries for one correlation id.
- Reader returns all entries for that correlation id.
- Reader distinguishes no entry from a `missing_artifact` marker.
- Reader preserves artifact kind and producer in returned entries.

### 3. Existing evidence source integration tests

Goal: prove the index links existing artifact streams without replacing them.

Candidate existing sources:

- record-session JSONL from `record-session-contract-and-watch` flows.
- `.ralph/events.jsonl` from `EventLogger`.
- runtime delivery / lifecycle durable records in `crates/ralph-core/src/event_logger.rs`.
- capability invocation artifacts from `crates/ralph-cli/src/capability.rs`.

Planned assertions:

- A record-session artifact can be registered and resolved by session/run correlation.
- A runtime delivery or lifecycle artifact can be registered and resolved by event/delivery correlation.
- A capability invocation id resolves to `invoke.json` plus `result.json` or `failed.json` entries.
- Existing artifact files remain parseable directly; index is not the only readable source.

### 4. Missing artifact tests

Goal: make absence auditable.

Planned assertions:

- Registering a missing artifact marker creates a lookup result with `status=missing`.
- Missing marker includes expected artifact kind and producer.
- Missing marker is distinguishable from no index entry.

### 5. Parent-child link tests

Goal: support isolated child run / micro-run without hot topology mutation.

Planned assertions:

- Parent invocation correlation id links to child result/failure correlation id.
- Child artifact lookup can point back to parent invocation.
- Parent-child link does not require changing live `HatRegistry`, `EventLoop`, or supervisor topology.

### 6. Phase boundary guardrail tests

Goal: prevent Phase 1B from leaking into Phase 1A.

Planned assertions:

- Contract tests do not invoke `ralph evidence summary`.
- Contract tests do not invoke `ralph evidence inspect`.
- Contract tests do not invoke `ralph doctor evidence`.
- Index schema does not require diagnosis taxonomy fields.
- Runtime graph / Rerun output is not used as the durable truth source for index lookup.

## Focused verification commands after implementation approval

The exact test names will be finalized during implementation. Expected command shape:

```bash
cargo test --package ralph-core --lib evidence_index -- --exact
cargo test --package ralph-core --lib event_logger::tests::test_runtime_durable_payloads_are_not_truncated -- --exact
cargo test --package ralph-core --lib session_recorder::tests::test_record_session_critical_sequence_strict_parseable_after_flush -- --exact
cargo test --package ralph-cli --bin ralph capability::tests::isolated_invocation_writes_auditable_artifacts_without_parent_topology_mutation -- --exact
openspec validate runtime-evidence-index-kernel --type change
```

## Stop conditions

Stop implementation and return to planning if:

- Minimal schema needs evidence CLI / doctor display fields to work.
- Index cannot represent record-session, runtime delivery, reply, and capability invocation with the same entry model.
- Tests require treating runtime graph layout as durable truth.
- Parent-child link requires live topology mutation.
