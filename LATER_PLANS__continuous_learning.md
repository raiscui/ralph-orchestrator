## [2026-04-30 09:25:00] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] 主题: 后续迁移 archive 根层旧平铺文件

### 延后事项

- 当前 `archive/` 根层仍有早期平铺的默认历史文件,例如 `task_plan_*.md`, `notes_*.md`, `WORKLOG_*.md`, `ERRORFIX_*.md`。
- 本轮只归档了实际检索覆盖到的旧支线,并把超过 1000 行的默认 `notes.md` 续档到 `archive/default_history/`。
- 后续可以单独做一次 archive 结构整理:
  - 先读取已有 `archive/manifests/`。
  - 再把根层默认历史文件迁移到 `archive/default_history/`。
  - 如果发现支线历史快照,按主题迁移到 `archive/branch_contexts/<topic>/snapshots/<timestamp>/`。

### 为什么先记下来

- 这属于仓库长期上下文卫生,但不是本轮持续学习的主要目标。
- 如果现在扩大到全 archive 重排,会影响大量旧路径,也需要额外引用扫描。
- 先记录下来,避免以后误以为根层平铺文件是本轮遗漏。
