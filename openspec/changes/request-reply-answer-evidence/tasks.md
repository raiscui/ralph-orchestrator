## 1. OpenSpec artifacts

- [x] 1.1 Create proposal for `request-reply-answer-evidence`.
- [x] 1.2 Create design for minimal answer-return evidence linkage.
- [x] 1.3 Create delta spec for request/reply answer evidence.
- [x] 1.4 Create test plan.
- [x] 1.5 Validate the change with `openspec validate request-reply-answer-evidence --type change`.

## 2. Implementation preparation after approval

- [ ] 2.1 Map exact supervisor routing write points for success and fail-closed answer-return evidence.
- [ ] 2.2 Decide whether evidence writes happen directly in supervisor routing or through a small helper.
- [ ] 2.3 Decide how answer timeout/missing markers are triggered without adding a broad broker.

## 3. Contract tests to implement after approval

- [ ] 3.1 Add successful `reply.hat.message` evidence index tests.
- [ ] 3.2 Add unknown request id fail-closed evidence tests.
- [ ] 3.3 Add no `source_instance` fail-closed evidence tests.
- [ ] 3.4 Add missing/timeout marker tests.
- [ ] 3.5 Add guardrail test proving ordinary workflow events with `reply` are not answer-return evidence.
- [ ] 3.6 Add guardrail test proving `reply.hat.message` does not automatically publish `reply.human.message`.

## 4. Verification to run after implementation approval

- [ ] 4.1 Run focused supervisor routing tests.
- [ ] 4.2 Run focused evidence index tests.
- [ ] 4.3 Run `cargo test -p ralph-core smoke_runner` if runtime routing code changes.
- [ ] 4.4 Run `cargo test` before declaring implementation complete.
- [ ] 4.5 Run `openspec validate request-reply-answer-evidence --type change` after code changes.
