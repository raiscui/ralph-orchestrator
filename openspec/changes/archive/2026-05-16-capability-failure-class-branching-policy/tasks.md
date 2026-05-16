## 1. Spec and design
- [x] 1.1 Create proposal for `capability-failure-class-branching-policy`.
- [x] 1.2 Create design for structured failure class as parent branching input.
- [x] 1.3 Add delta spec for `failure_class` and parent branching policy.
- [x] 1.4 Validate the change with `openspec validate capability-failure-class-branching-policy --type change`.

## 2. Implementation
- [x] 2.1 Add structured failure class types to runtime capability records.
- [x] 2.2 Populate failure classes in core parent failure paths.
- [x] 2.3 Populate failure classes in CLI capability invocation failure paths.
- [x] 2.4 Add focused unit assertions for failure classification.

## 3. Dogfood and verification
- [x] 3.1 Strengthen the failure fallback live gate to require `failure_class=invalid_capability_id`.
- [x] 3.2 Verify core capability tests.
- [x] 3.3 Verify CLI capability tests.
- [x] 3.4 Verify the live failure fallback integration gate.
- [x] 3.5 Run repo-wide verification before archive.

## 4. Archive and sync
- [x] 4.1 Archive the change.
- [x] 4.2 Confirm stable spec sync.
- [x] 4.3 Record notes / worklog / task plan completion.
- [x] 4.4 Create a local commit.
