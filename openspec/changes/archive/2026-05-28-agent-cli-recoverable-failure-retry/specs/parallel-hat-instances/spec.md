## ADDED Requirements

### Requirement: Parallel jobs distinguish recoverable CLI failures
The parallel runtime MUST distinguish classified recoverable agent CLI failures from ordinary terminal job failures.

When a HatJobResult-like outcome is classified as recoverable, the affected instance/job lifecycle MUST enter a retry-aware state such as `retry_scheduled`, `paused_recoverable`, or `retrying` instead of immediately collapsing into ordinary terminal failure.

#### Scenario: Rate-limited worker enters retry-aware state
- **WHEN** `writer#1` fails a headless CLI job with stderr containing `429 Too Many Requests` and the classifier marks it recoverable
- **THEN** the parallel runtime MUST record recoverable failure evidence for `writer#1`
- **THEN** `writer#1` MUST remain represented as retry-aware rather than disappearing as an ordinary failed worker

### Requirement: Retryable jobs block workflow completion while pending
The parallel runtime MUST NOT declare a workflow complete while any recoverable failure remains pending, paused, scheduled, or retrying.

A recoverable job lifecycle MUST resolve to either success after retry, explicit exhaustion, cancellation, or another terminal state before coordinator-controlled completion can be accepted.

#### Scenario: Completion candidate waits for pending retry
- **GIVEN** a dynamic worker has a recoverable failure with status `retry_scheduled`
- **WHEN** the coordinator observes a workflow completion candidate event
- **THEN** the parallel runtime MUST keep the run open or gate completion until the retry lifecycle resolves

### Requirement: Exhausted recoverable failures become auditable terminal failures
The parallel runtime MUST convert exhausted recoverable failures into terminal job failures with a pointer to recoverable-failure ledger evidence.

The terminal job result MUST preserve enough information for evidence inspection, reports, and agents snapshots to explain that the job ended after retry exhaustion rather than an ordinary first-attempt process failure.

#### Scenario: Exhausted retry is visible in job evidence
- **GIVEN** a job has reached the configured maximum recoverable retry attempts
- **WHEN** the final attempt fails with the same recoverable failure kind
- **THEN** the runtime MUST mark the job terminal
- **THEN** the terminal evidence MUST include the `failure_id` or ledger location for the exhausted retry lifecycle

### Requirement: Retry execution preserves stdout-only event parsing
The parallel runtime MUST preserve the existing stdout-only workflow event parsing boundary during recoverable retry handling.

Recoverable failure classification MAY inspect bounded stderr evidence for retryability, but workflow event extraction MUST continue to use stdout event output only.

#### Scenario: Stderr classification does not create workflow events
- **WHEN** a worker process fails and writes a recoverable failure message to stderr
- **THEN** the runtime MAY classify the stderr for retry scheduling
- **THEN** the runtime MUST NOT add stderr text to `output_for_parsing` or parse it as workflow events
