## ADDED Requirements

### Requirement: Parent-visible capability failures MUST include a structured failure class
Ralph MUST include a structured `failure_class` in parent-visible `capability.failed` payloads.

The `failure_class` MUST be the preferred parent branching input for capability failure handling. Parent policy MUST be able to depend on that field instead of parsing free-form error strings.

#### Scenario: invalid capability id is classified before fallback
- **GIVEN** a parent run emits a `capability.request` with an invalid capability id
- **WHEN** the runtime returns `capability.failed`
- **THEN** the failure payload MUST include `failure_class = invalid_capability_id`
- **AND** the parent MUST be able to see that structured class in a later turn before choosing fallback behavior

#### Scenario: child execution failure remains distinguishable from pre-invocation failure
- **GIVEN** an isolated capability child or micro-run starts and later fails
- **WHEN** the runtime returns failure records or artifacts
- **THEN** the failure MUST remain distinguishable from pre-invocation selection failures through a structured failure class
- **AND** any created invocation id or failure artifact links MUST remain auditable

### Requirement: Parent branching policy MUST prefer structured failure class over free-form error parsing
Ralph MUST preserve a product contract where parent-side capability branching decisions can be driven by structured failure classification.

Free-form `error` text MAY still be present for human diagnosis, but it MUST NOT be the only stable signal available for parent orchestration.

#### Scenario: fallback branch keys off invalid capability class
- **GIVEN** a parent run receives `capability.failed` with `failure_class = invalid_capability_id`
- **WHEN** the parent decides how to continue
- **THEN** it MUST be able to emit an explicit fallback capability request based on that structured class
- **AND** the later fallback success and final `reply.human.message` MUST remain separately auditable
