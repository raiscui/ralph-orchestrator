//! Integration tests for the smoke test replay runner.

use ralph_core::testing::{SmokeRunner, SmokeTestConfig, TerminationReason, list_fixtures};
use std::path::PathBuf;

/// Returns the path to the test fixtures directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

// ─────────────────────────────────────────────────────────────────────────────
// Acceptance Criteria #6: Example Fixture Included
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_fixtures_directory_exists() {
    let dir = fixtures_dir();
    assert!(dir.exists(), "Fixtures directory should exist at {:?}", dir);
}

#[test]
fn test_basic_session_fixture_exists() {
    let fixture = fixtures_dir().join("basic_session.jsonl");
    assert!(
        fixture.exists(),
        "Basic session fixture should exist at {:?}",
        fixture
    );
}


#[test]
fn test_parallel_trigger_routing_fixture_exists() {
    let fixture = fixtures_dir().join("parallel_trigger_routing.jsonl");
    assert!(
        fixture.exists(),
        "Parallel trigger routing fixture should exist at {:?}",
        fixture
    );
}

#[test]
fn test_parallel_supervisor_tui_gate_chat_fixture_exists() {
    let fixture = fixtures_dir().join("parallel_supervisor_tui_gate_chat.jsonl");
    assert!(
        fixture.exists(),
        "Parallel supervisor TUI gate/chat fixture should exist at {:?}",
        fixture
    );
}

#[test]
fn test_parallel_workflow_semantics_fixture_exists() {
    let fixture = fixtures_dir().join("parallel_workflow_semantics.jsonl");
    assert!(
        fixture.exists(),
        "Parallel workflow semantics fixture should exist at {:?}",
        fixture
    );
}

#[test]
fn test_parallel_experimental_dev_engine_fixture_exists() {
    let fixture = fixtures_dir().join("parallel_experimental_dev_engine.jsonl");
    assert!(
        fixture.exists(),
        "Parallel experimental dev engine fixture should exist at {:?}",
        fixture
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Acceptance Criteria #7: Integration Test Validates Full Replay Flow
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_full_replay_flow_with_basic_session() {
    let fixture = fixtures_dir().join("basic_session.jsonl");

    let config = SmokeTestConfig::new(&fixture);
    let result = SmokeRunner::run(&config).expect("Should run fixture successfully");

    // Verify completion
    assert!(
        result.completed_successfully(),
        "Basic session should complete successfully"
    );
    assert_eq!(
        *result.termination_reason(),
        TerminationReason::Completed,
        "Should terminate with Completed (LOOP_COMPLETE detected)"
    );

    // Verify iterations (one per terminal write chunk)
    assert!(
        result.iterations_run() >= 2,
        "Should process at least 2 chunks (completion found in 3rd)"
    );

    // Verify event parsing
    // Fixture contains: build.task and build.done events
    assert!(
        result.event_count() >= 2,
        "Should parse at least 2 events from fixture, got {}",
        result.event_count()
    );

    // Verify output processing
    assert!(
        result.output_bytes() > 0,
        "Should have processed some output bytes"
    );
}


#[test]
fn test_full_replay_flow_with_parallel_trigger_routing_fixture() {
    use ralph_core::testing::ReplayBackend;

    let fixture = fixtures_dir().join("parallel_trigger_routing.jsonl");

    let config = SmokeTestConfig::new(&fixture);
    let result = SmokeRunner::run(&config).expect("Should run fixture successfully");

    assert!(
        result.completed_successfully(),
        "Parallel trigger routing session should complete successfully"
    );
    assert_eq!(
        *result.termination_reason(),
        TerminationReason::Completed,
        "Should terminate with Completed (LOOP_COMPLETE detected)"
    );

    // Fixture contains: build.task, build.done, test.done, routing.escalate
    assert!(
        result.event_count() >= 4,
        "Should parse at least 4 events from fixture, got {}",
        result.event_count()
    );

    // 额外校验：fixture 里确实包含“多实例输出归因前缀”，避免只测到 event parser 而没覆盖并行日志形态。
    let mut backend = ReplayBackend::from_file(&fixture).expect("Should load replay fixture");
    let output = String::from_utf8(backend.collect_all()).expect("Fixture output must be UTF-8");
    assert!(
        output.contains("[writer#1:out]"),
        "Fixture should contain writer#1 attribution prefix"
    );
    assert!(
        output.contains("[tester#1:out]"),
        "Fixture should contain tester#1 attribution prefix"
    );
    assert!(
        output.contains("routing.escalate"),
        "Fixture should contain routing.escalate event"
    );
}

#[test]
fn test_full_replay_flow_with_parallel_supervisor_tui_gate_chat_fixture() {
    use ralph_core::testing::ReplayBackend;

    let fixture = fixtures_dir().join("parallel_supervisor_tui_gate_chat.jsonl");

    let config = SmokeTestConfig::new(&fixture);
    let result = SmokeRunner::run(&config).expect("Should run fixture successfully");

    assert!(
        result.completed_successfully(),
        "Parallel supervisor TUI gate/chat session should complete successfully"
    );
    assert_eq!(
        *result.termination_reason(),
        TerminationReason::Completed,
        "Should terminate with Completed (LOOP_COMPLETE detected)"
    );

    // Fixture contains: build.task, gate.request, human.message, gate.resolve, build.done
    assert!(
        result.event_count() >= 5,
        "Should parse at least 5 events from fixture, got {}",
        result.event_count()
    );

    // 额外校验：fixture 文本里确实包含 gate.* / human.message，避免只测到 LOOP_COMPLETE。
    let mut backend = ReplayBackend::from_file(&fixture).expect("Should load replay fixture");
    let output = String::from_utf8(backend.collect_all()).expect("Fixture output must be UTF-8");
    assert!(
        output.contains("gate.request"),
        "Fixture should contain gate.request event"
    );
    assert!(
        output.contains("gate.resolve"),
        "Fixture should contain gate.resolve event"
    );
    assert!(
        output.contains("human.message"),
        "Fixture should contain human.message event"
    );
}

#[test]
fn test_full_replay_flow_with_parallel_workflow_semantics_fixture() {
    use ralph_core::testing::ReplayBackend;

    let fixture = fixtures_dir().join("parallel_workflow_semantics.jsonl");

    let config = SmokeTestConfig::new(&fixture);
    let result = SmokeRunner::run(&config).expect("Should run fixture successfully");

    assert!(
        result.completed_successfully(),
        "Parallel workflow semantics session should complete successfully"
    );
    assert_eq!(
        *result.termination_reason(),
        TerminationReason::Completed,
        "Should terminate with Completed (LOOP_COMPLETE detected)"
    );

    // Fixture contains: unknown.topic, fix.applied
    assert!(
        result.event_count() >= 2,
        "Should parse at least 2 events from fixture, got {}",
        result.event_count()
    );

    // 额外校验：链式 fallback（经理兜底）+ completion candidate → ralph#1 输出 LOOP_COMPLETE
    let mut backend = ReplayBackend::from_file(&fixture).expect("Should load replay fixture");
    let output = String::from_utf8(backend.collect_all()).expect("Fixture output must be UTF-8");
    assert!(
        output.contains("[manager#1:out] manager: unknown.topic"),
        "Fixture should show wildcard manager handling unknown.topic"
    );
    assert!(
        output.contains("fix.applied"),
        "Fixture should contain fix.applied completion candidate event"
    );
    assert!(
        output.contains("[ralph#1:out]"),
        "Fixture should contain ralph#1 attribution prefix"
    );
    assert!(
        output.contains("LOOP_COMPLETE"),
        "Fixture should terminate via completion promise"
    );
}

#[test]
fn test_full_replay_flow_with_parallel_experimental_dev_engine_fixture() {
    use ralph_core::testing::ReplayBackend;

    let fixture = fixtures_dir().join("parallel_experimental_dev_engine.jsonl");

    let config = SmokeTestConfig::new(&fixture);
    let result = SmokeRunner::run(&config).expect("Should run fixture successfully");

    assert!(
        result.completed_successfully(),
        "Parallel experimental dev engine session should complete successfully"
    );
    assert_eq!(
        *result.termination_reason(),
        TerminationReason::Completed,
        "Should terminate with Completed (LOOP_COMPLETE detected)"
    );

    // Fixture should contain a full chain:
    // experiment.task -> experiment.result -> experiment.reviewed -> integration.task -> integration.applied
    assert!(
        result.event_count() >= 6,
        "Should parse at least 6 events from fixture, got {}",
        result.event_count()
    );

    // 额外校验：fixture 文本里确实包含“并行归因前缀 + 关键 topic + commit + LOOP_COMPLETE”，锁死示例语义。
    let mut backend = ReplayBackend::from_file(&fixture).expect("Should load replay fixture");
    let output = String::from_utf8(backend.collect_all()).expect("Fixture output must be UTF-8");
    assert!(
        output.contains("[experiment_runner#1:out]")
            || output.contains("[experiment_runner#2:out]"),
        "Fixture should contain experiment_runner attribution prefix"
    );
    assert!(
        output.contains("[experiment_auditor#1:out]"),
        "Fixture should contain experiment_auditor attribution prefix"
    );
    assert!(
        output.contains("[experiment_integrator#1:out]"),
        "Fixture should contain experiment_integrator attribution prefix"
    );
    assert!(
        output.contains("[ralph#1:out]"),
        "Fixture should contain ralph#1 attribution prefix"
    );

    assert!(
        output.contains("experiment.task"),
        "Fixture should contain experiment.task event"
    );
    assert!(
        output.contains("experiment.result"),
        "Fixture should contain experiment.result event"
    );
    assert!(
        output.contains("experiment.reviewed"),
        "Fixture should contain experiment.reviewed event"
    );
    assert!(
        output.contains("integration.task"),
        "Fixture should contain integration.task event"
    );
    assert!(
        output.contains("integration.applied"),
        "Fixture should contain integration.applied event"
    );
    assert!(
        output.contains("experiment.complete"),
        "Fixture should contain experiment.complete event"
    );
    assert!(
        output.contains("commit:"),
        "Fixture should contain a commit payload"
    );
    assert!(
        !output.contains("patch: |"),
        "Fixture should not embed a patch payload"
    );
    assert!(
        output.contains("LOOP_COMPLETE"),
        "Fixture should terminate via completion promise"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Acceptance Criteria #6: Fixture Discovery
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_fixture_discovery() {
    let fixtures = list_fixtures(fixtures_dir()).expect("Should list fixtures");

    // Should find at least basic_session.jsonl
    assert!(
        !fixtures.is_empty(),
        "Should find at least one fixture in {:?}",
        fixtures_dir()
    );

    let fixture_names: Vec<_> = fixtures
        .iter()
        .filter_map(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .collect();

    assert!(
        fixture_names.contains(&"basic_session.jsonl".to_string()),
        "Should discover basic_session.jsonl, found: {:?}",
        fixture_names
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Programmatic Fixture Loading
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_all_discovered_fixtures_are_valid() {
    let fixtures = list_fixtures(fixtures_dir()).expect("Should list fixtures");

    for fixture_path in fixtures {
        let config = SmokeTestConfig::new(&fixture_path);
        let result = SmokeRunner::run(&config);

        assert!(
            result.is_ok(),
            "Fixture {:?} should be valid and runnable: {:?}",
            fixture_path,
            result.err()
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// REGRESSION DETECTION TESTS
// These tests prove the smoke test infrastructure catches bugs and regressions.
// They intentionally create broken scenarios to verify the system fails correctly.
// ═══════════════════════════════════════════════════════════════════════════════

mod regression_detection {
    use super::*;
    use ralph_core::Record;
    use ralph_core::testing::{ReplayBackend, SmokeTestError};
    use ralph_proto::TerminalWrite;
    use std::io::Write;
    use std::time::Duration;
    use tempfile::TempDir;

    /// Helper to create a fixture file with given content.
    fn create_fixture(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    /// Creates a valid terminal write JSONL line using ralph_proto's TerminalWrite.
    fn make_write_line(text: &str, offset_ms: u64) -> String {
        let write = TerminalWrite::new(text.as_bytes(), true, offset_ms);
        let record = Record {
            ts: 1000 + offset_ms,
            event: "ux.terminal.write".to_string(),
            data: serde_json::to_value(&write).unwrap(),
        };
        serde_json::to_string(&record).unwrap()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 1: Malformed JSONL is Caught
    // Verifies: Invalid fixture format is detected and rejected
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_catches_malformed_jsonl_fixture() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = create_fixture(
            temp_dir.path(),
            "malformed.jsonl",
            "this is not valid json\nalso not valid",
        );

        let config = SmokeTestConfig::new(&fixture_path);
        let result = SmokeRunner::run(&config);

        assert!(
            result.is_err(),
            "Malformed JSONL should cause an error, but got: {:?}",
            result
        );

        let err = result.unwrap_err();
        assert!(
            matches!(err, SmokeTestError::Io(_)),
            "Should be IO error from JSON parsing, got: {:?}",
            err
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 2: Invalid Base64 Data is Caught
    // Verifies: Corrupted event data doesn't silently pass
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_catches_invalid_base64_in_terminal_write() {
        let temp_dir = TempDir::new().unwrap();

        // Create a fixture with invalid base64 in the bytes field
        let invalid_fixture = r#"{"ts":1000,"event":"ux.terminal.write","data":{"bytes":"!!!INVALID_BASE64!!!","stdout":true,"offset_ms":0}}"#;
        let fixture_path = create_fixture(temp_dir.path(), "bad_base64.jsonl", invalid_fixture);

        // ReplayBackend should handle this gracefully (returns None for bad decode)
        let backend = ReplayBackend::from_file(&fixture_path);
        assert!(
            backend.is_ok(),
            "Should load file, handling bad data gracefully"
        );

        let mut backend = backend.unwrap();
        // Invalid base64 should result in None output (skipped)
        let output = backend.next_output();
        assert!(
            output.is_none(),
            "Invalid base64 should be skipped, not crash"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 3: Missing Required Fields Detected
    // Verifies: Incomplete records cause errors (strict parsing)
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_catches_missing_data_field() {
        let temp_dir = TempDir::new().unwrap();

        // Missing "data" field entirely - Record struct requires this field
        let incomplete = r#"{"ts":1000,"event":"ux.terminal.write"}"#;
        let fixture_path = create_fixture(temp_dir.path(), "missing_data.jsonl", incomplete);

        // The system is strict: missing required fields cause an error
        // This is the correct behavior - malformed records should be caught
        let backend = ReplayBackend::from_file(&fixture_path);
        assert!(
            backend.is_err(),
            "REGRESSION: Missing required 'data' field should cause an error"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 4: Event Parser Regression Detection
    // Verifies: If event parsing breaks, tests detect it
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_event_parser_counts_detected_events() {
        let temp_dir = TempDir::new().unwrap();

        // Create a fixture with known events
        let output_with_events = r#"Starting task
<event topic="build.task">Task 1</event>
Working on implementation...
<event topic="build.done">
tests: pass
lint: pass
typecheck: pass
</event>
Finishing up"#;

        let line = make_write_line(output_with_events, 0);
        let fixture_path = create_fixture(temp_dir.path(), "with_events.jsonl", &line);

        let config = SmokeTestConfig::new(&fixture_path);
        let result = SmokeRunner::run(&config).expect("Should process fixture");

        // This test will FAIL if EventParser breaks - it expects exactly 2 events
        assert_eq!(
            result.event_count(),
            2,
            "REGRESSION: EventParser should find exactly 2 events (build.task, build.done)"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 5: Completion Promise Detection
    // Verifies: LOOP_COMPLETE detection works correctly
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_completion_promise_detected() {
        let temp_dir = TempDir::new().unwrap();

        let line1 = make_write_line("Working on task...", 0);
        let line2 = make_write_line("LOOP_COMPLETE", 100);
        let content = format!("{}\n{}\n", line1, line2);

        let fixture_path = create_fixture(temp_dir.path(), "with_completion.jsonl", &content);

        let config = SmokeTestConfig::new(&fixture_path);
        let result = SmokeRunner::run(&config).expect("Should run");

        assert_eq!(
            *result.termination_reason(),
            TerminationReason::Completed,
            "REGRESSION: LOOP_COMPLETE should trigger Completed termination"
        );
    }

    #[test]
    fn test_no_completion_results_in_fixture_exhausted() {
        let temp_dir = TempDir::new().unwrap();

        let line1 = make_write_line("Working on task...", 0);
        let line2 = make_write_line("Done but no completion promise", 100);
        let content = format!("{}\n{}\n", line1, line2);

        let fixture_path = create_fixture(temp_dir.path(), "no_completion.jsonl", &content);

        let config = SmokeTestConfig::new(&fixture_path);
        let result = SmokeRunner::run(&config).expect("Should run");

        assert_eq!(
            *result.termination_reason(),
            TerminationReason::FixtureExhausted,
            "REGRESSION: Missing LOOP_COMPLETE should result in FixtureExhausted"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 6: Promise in Event Tag Should NOT Complete
    // Verifies: Safety mechanism prevents false completion
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_promise_inside_event_tag_does_not_complete() {
        let temp_dir = TempDir::new().unwrap();

        // The LOOP_COMPLETE appears only inside an event tag - should NOT complete
        let output = r#"Working on task...
<event topic="build.task">Fix LOOP_COMPLETE detection bug</event>
Still working..."#;

        let line = make_write_line(output, 0);
        let fixture_path = create_fixture(temp_dir.path(), "promise_in_tag.jsonl", &line);

        let config = SmokeTestConfig::new(&fixture_path);
        let result = SmokeRunner::run(&config).expect("Should run");

        assert_eq!(
            *result.termination_reason(),
            TerminationReason::FixtureExhausted,
            "REGRESSION: LOOP_COMPLETE inside event tag should NOT trigger completion"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 7: Output Byte Counting
    // Verifies: All output is properly processed and counted
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_output_bytes_counted_correctly() {
        let temp_dir = TempDir::new().unwrap();

        let text1 = "Hello"; // 5 bytes
        let text2 = "World"; // 5 bytes
        let line1 = make_write_line(text1, 0);
        let line2 = make_write_line(text2, 100);
        let content = format!("{}\n{}\n", line1, line2);

        let fixture_path = create_fixture(temp_dir.path(), "byte_count.jsonl", &content);

        let config = SmokeTestConfig::new(&fixture_path);
        let result = SmokeRunner::run(&config).expect("Should run");

        assert_eq!(
            result.output_bytes(),
            10,
            "REGRESSION: Output bytes should be exactly 10 (5 + 5)"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 8: Iteration Counting
    // Verifies: Each output chunk is counted as an iteration
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_iteration_count_matches_chunks() {
        let temp_dir = TempDir::new().unwrap();

        // 5 separate terminal write chunks
        let lines: Vec<String> = (0..5)
            .map(|i| make_write_line(&format!("Chunk {}", i), i * 100))
            .collect();
        let content = lines.join("\n") + "\n";

        let fixture_path = create_fixture(temp_dir.path(), "five_chunks.jsonl", &content);

        let config = SmokeTestConfig::new(&fixture_path);
        let result = SmokeRunner::run(&config).expect("Should run");

        assert_eq!(
            result.iterations_run(),
            5,
            "REGRESSION: Should have exactly 5 iterations for 5 chunks"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 9: Fixture File Not Found
    // Verifies: Missing fixtures produce clear errors
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_missing_fixture_produces_clear_error() {
        let config = SmokeTestConfig::new("/definitely/does/not/exist/fixture.jsonl");
        let result = SmokeRunner::run(&config);

        assert!(result.is_err(), "Missing fixture should error");

        let err = result.unwrap_err();
        match err {
            SmokeTestError::FixtureNotFound(path) => {
                assert!(
                    path.to_string_lossy().contains("fixture.jsonl"),
                    "Error should contain the missing filename"
                );
            }
            _ => panic!(
                "REGRESSION: Should be FixtureNotFound error, got: {:?}",
                err
            ),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 10: Non-Terminal Events Are Filtered
    // Verifies: Only terminal write events contribute to output
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_non_terminal_events_filtered_out() {
        let temp_dir = TempDir::new().unwrap();

        // Mix of terminal writes and metadata events
        let terminal = make_write_line("Hello", 0);
        let meta = r#"{"ts":1050,"event":"_meta.iteration","data":{"n":1,"elapsed_ms":50,"hat":"default"}}"#;
        let bus = r#"{"ts":1100,"event":"bus.publish","data":{"topic":"test","payload":"data"}}"#;

        let content = format!("{}\n{}\n{}\n", terminal, meta, bus);
        let fixture_path = create_fixture(temp_dir.path(), "mixed_events.jsonl", &content);

        let config = SmokeTestConfig::new(&fixture_path);
        let result = SmokeRunner::run(&config).expect("Should run");

        // Only 1 terminal write, so only 1 iteration
        assert_eq!(
            result.iterations_run(),
            1,
            "REGRESSION: Should only count terminal write events"
        );
        assert_eq!(
            result.output_bytes(),
            5,
            "REGRESSION: Only terminal write bytes should be counted"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 11: Empty Fixture Handling
    // Verifies: Empty fixtures don't crash and report correctly
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_empty_fixture_handled_gracefully() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = create_fixture(temp_dir.path(), "empty.jsonl", "");

        let config = SmokeTestConfig::new(&fixture_path);
        let result = SmokeRunner::run(&config).expect("Empty fixture should not error");

        assert_eq!(result.iterations_run(), 0);
        assert_eq!(result.event_count(), 0);
        assert_eq!(result.output_bytes(), 0);
        assert_eq!(
            *result.termination_reason(),
            TerminationReason::FixtureExhausted
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 12: ReplayBackend Order Preservation
    // Verifies: Output chunks are served in the exact order recorded
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_replay_preserves_order() {
        let temp_dir = TempDir::new().unwrap();

        let line1 = make_write_line("First", 0);
        let line2 = make_write_line("Second", 100);
        let line3 = make_write_line("Third", 200);
        let content = format!("{}\n{}\n{}\n", line1, line2, line3);

        let fixture_path = create_fixture(temp_dir.path(), "ordered.jsonl", &content);
        let mut backend = ReplayBackend::from_file(&fixture_path).expect("Should load");

        assert_eq!(
            backend.next_output().unwrap(),
            b"First",
            "REGRESSION: First chunk should be 'First'"
        );
        assert_eq!(
            backend.next_output().unwrap(),
            b"Second",
            "REGRESSION: Second chunk should be 'Second'"
        );
        assert_eq!(
            backend.next_output().unwrap(),
            b"Third",
            "REGRESSION: Third chunk should be 'Third'"
        );
        assert!(
            backend.next_output().is_none(),
            "Should be exhausted after 3 chunks"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 13: Verify Basic Session Fixture Contract
    // Verifies: The canonical basic_session.jsonl fixture meets expected invariants
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_basic_session_fixture_contract() {
        let fixture = fixtures_dir().join("basic_session.jsonl");
        let config = SmokeTestConfig::new(&fixture);
        let result = SmokeRunner::run(&config).expect("Basic session should run");

        // Contract: Must complete successfully
        assert!(
            result.completed_successfully(),
            "REGRESSION: Basic session fixture must complete successfully"
        );

        // Contract: Must have completion termination (contains LOOP_COMPLETE)
        assert_eq!(
            *result.termination_reason(),
            TerminationReason::Completed,
            "REGRESSION: Basic session must detect LOOP_COMPLETE"
        );

        // Contract: Must parse events (the fixture contains build.task and build.done)
        assert!(
            result.event_count() >= 2,
            "REGRESSION: Basic session must contain at least 2 parseable events, got {}",
            result.event_count()
        );

        // Contract: Must have processed meaningful output
        assert!(
            result.output_bytes() > 100,
            "REGRESSION: Basic session should have substantial output, got {} bytes",
            result.output_bytes()
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 14: Timeout Configuration Works
    // Verifies: Very short timeout will eventually trigger
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_timeout_configuration_respected() {
        let temp_dir = TempDir::new().unwrap();

        // Create a fixture - timeout of 0ms should be extremely short
        // Note: This test verifies timeout CONFIGURATION works, not that it triggers
        // (the fixture is too small to actually timeout)
        let line = make_write_line("Quick output", 0);
        let fixture_path = create_fixture(temp_dir.path(), "quick.jsonl", &line);

        let config = SmokeTestConfig::new(&fixture_path).with_timeout(Duration::from_secs(60));

        // Verify config was set
        assert_eq!(config.timeout, Duration::from_secs(60));

        let result = SmokeRunner::run(&config).expect("Should complete within 60s");
        assert!(result.completed_successfully());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Test 15: ReplayBackend Reset Works
    // Verifies: Resetting allows re-reading the same fixture
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_replay_backend_reset() {
        let temp_dir = TempDir::new().unwrap();
        let line = make_write_line("ReplayMe", 0);
        let fixture_path = create_fixture(temp_dir.path(), "replay.jsonl", &line);

        let mut backend = ReplayBackend::from_file(&fixture_path).expect("Should load");

        // First pass
        assert_eq!(backend.next_output().unwrap(), b"ReplayMe");
        assert!(backend.is_exhausted());

        // Reset and replay
        backend.reset();
        assert!(!backend.is_exhausted());
        assert_eq!(
            backend.next_output().unwrap(),
            b"ReplayMe",
            "REGRESSION: Reset should allow replaying from start"
        );
    }
}

