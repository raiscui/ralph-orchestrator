# 任务计划: 1 + 2 + 3(落 proposal Appendix C + Group 4 重分类 + 新 change)

## 状态: 三项全部完成

### 落盘

- proposal.md 加 Appendix C(89 行),从 433 → 522
- tasks.md 4.4 改 [x] dropped + 4.15 新增 dropped
- 新建 `openspec/changes/declarative-e2e-mock-parity/`
  - proposal.md 125 行
  - tasks.md 59 行
- (audit-p3-p4.md + audit report 在 change 目录内,207 行,完整)

### 决定

- [决定]: 4.4 + 4.15 dropped,因为目标文件 `mcp.rs` 与整个 ralph-api/ crate 已删
- [决定]: 新的 declarative-e2e-mock-parity change 文件落在独立 change 目录
  [理由]: F1 是独立 concern(同步 mock),不混入 sync-origin-main-features
- [决定]: 新 change 用 「option A: 调用 imperative runner.configure_mock_mode」
  [理由]: imperative 已经做了硬失败 + persist_e2e_artifacts 等改进,dedupe 防止 drift

## 当前 HEAD

仍 `8b27556`(无新代码 commit)

## 工作树状态

- 不需要新 commit(纯文档)
- 等用户决策是否 commit proposal.md / tasks.md 改动
