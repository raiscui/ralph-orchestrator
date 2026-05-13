## 1. Scope Model And File Layout

- [x] 1.1 Define the canonical 5-scope model in core docs and migration notes
- [x] 1.2 Introduce path resolution helpers for instance context, role experience, and project experience
- [x] 1.3 Keep `.agent/memories.md` compatibility explicit while adding the new scoped experience entry points

## 2. Shared Experience Entry Protocol

- [x] 2.1 Design and implement one shared parser/serializer for role and project experience entries
- [x] 2.2 Add support for entry status, confidence, timestamps, and supersession metadata
- [x] 2.3 Add tests that load both role and project experience using the same protocol

## 3. Canonical Writer Enforcement

- [x] 3.1 Implement topic canonical writer ownership rules and reject non-owner shared writes
- [x] 3.2 Implement role canonical writer ownership rules for `.ralph/roles/<hat_id>/experience.md`
- [x] 3.3 Restrict project-root `experience.md` writes to `ralph#1` by default
- [x] 3.4 Add handoff summary support for topic and role writer transfers

## 4. Promotion And Demotion Flow

- [x] 4.1 Implement topic-to-role promotion evaluation rules
- [x] 4.2 Implement topic-to-project and role-to-project promotion evaluation rules
- [x] 4.3 Implement demotion/deprecation flow that preserves audit links instead of hard deletion
- [x] 4.4 Add regression tests for promotion and demotion decisions across the three reusable scopes

## 5. Injection And Read Policy

- [x] 5.1 Implement ordinary hat injection order for project, role, topic, instance, and runtime scopes
- [x] 5.2 Implement metadata-first injection flow for `ralph#1` before workflow or hat selection
- [x] 5.3 Implement summary-first, on-demand reads for topic and instance context
- [x] 5.4 Add tests that verify unrelated role experiences and historical topics are not eagerly injected

## 6. Migration, Tooling, And Documentation

- [x] 6.1 Document the scoped experience model, writer rules, and promotion ladder
- [x] 6.2 Document the current gap between documented `memories.path` and the actual implementation baseline
- [x] 6.3 Add CLI or doctor/debug visibility for active scoped experience paths and writer ownership where needed
- [x] 6.4 Run OpenSpec validation and update implementation guidance for follow-on changes such as startup bootstrap and runtime capability invocation
