## 1. Canonical bootstrap resource contract

- [x] 1.1 Define the canonical default bootstrap workflow resource and document its intended runtime fields.
- [x] 1.2 Update `resource-bootstrap` delta spec so implicit default bootstrap requires canonical field-level contract for `custom + codex + parallel` semantics.
- [x] 1.3 Decide and document the canonical resource naming and sync boundary with repository `ralph.yml`.

## 2. Selector and resource updates

- [x] 2.1 Add or replace the embedded startup workflow resource that represents the canonical default bootstrap configuration.
- [x] 2.2 Update `DEFAULT_BOOTSTRAP_WORKFLOW_ID` to select the canonical default bootstrap resource.
- [x] 2.3 Synchronize repository `ralph.yml` so its user-visible bootstrap runtime fields remain aligned with the canonical embedded resource.

## 3. Verification gates

- [x] 3.1 Update `startup_resources` focused tests to assert canonical selector choice and key `cli` / `parallel` field alignment.
- [x] 3.2 Update `integration_startup_resources` live gate to assert `.ralph/resolved-config.yml` matches canonical default bootstrap semantics for user-visible runtime fields.
- [x] 3.3 Add a repo-owned drift gate that compares root `ralph.yml` with the canonical embedded startup resource on the agreed bootstrap fields.
- [x] 3.4 Run OpenSpec validation plus focused startup bootstrap tests to prove the new contract.
