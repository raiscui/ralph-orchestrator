# Proposal: default-bootstrap-parallel-run

## Why

When a user runs `ralph run` in a workspace with no `ralph.yml` and no `PROMPT.md`, Ralph already enters startup resource bootstrap and writes `.ralph/resolved-config.yml`.

The product expectation is that this resolved default behaves like the default `ralph.yml` for the modern runtime: it should start in parallel mode, with `ralph#1` acting as coordinator.

## What Changes

This change makes the bootstrap-resolved config explicitly enable `parallel.enabled=true`.

## Goals

- Make no-config/no-prompt `ralph run` resolve to a parallel-mode startup config.
- Keep bootstrap selection before real `EventLoop` / parallel `Supervisor` initialization.
- Preserve explicit config behavior: `--config ...` must continue to bypass default bootstrap selection.
- Preserve startup-only artifact behavior: write `.ralph/bootstrap-selection.json` and `.ralph/resolved-config.yml`, not a workspace `ralph.yml`.

## Non-goals

- Do not hot-switch topology after runtime starts.
- Do not require writing a physical `ralph.yml` into the workspace.
- Do not change explicit missing config semantics.
