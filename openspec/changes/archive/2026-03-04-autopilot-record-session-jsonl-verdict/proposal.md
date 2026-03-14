## Why

目前要验证`/Users/cuiluming/local_doc/l_dev/my/rust/parallel-experimental-dev-engine`这类"并行实验开发"配置是否真的可用,常见做法是手动:

- 在该目录中运行`ralph run`.
- 用`--record-session`录制一份JSONL.
- 再靠人去翻stdout/stderr,翻`.ralph/events*.jsonl`.
- 最后人工判断是否走完闭环,以及结果是否符合程序设计要求.

这套流程的问题是:

- 不能无人值守: 一旦出现`gate.*`或行为漂移,就会卡住,或需要人肉判定.
- 证据分散: 录制的JSONL才是最终成果.
  但目前缺少"以JSONL为唯一证据源"的自动判定与报告生成.
- 环境约束强: 该目录本身是一个已初始化的Git仓库.
  并行runner依赖`git worktree`来隔离job.
  因此测试机制不能随意复制或临时创建目录.
  机制必须支持"指定一个已经初始化好的repo目录,并在其内就地运行".

我们需要一套"无人干预 autopilot"能力.
它要把上述过程固化成一条命令.
它还要把最终判定建立在`--record-session`生成的JSONL之上.
这样才能被定时任务或CI稳定调用.

## What Changes

- 新增`ralph autopilot`命令族(不改变既有`ralph run`语义):
  - `ralph autopilot run`: 在指定Git仓库目录内执行一次`ralph run --record-session <file>`.
    同时落盘辅助证据.
    结束后分析录制JSONL,给出verdict与退出码.
  - `ralph autopilot analyze`: 不重新运行.
    只分析一份已存在的`--record-session` JSONL.
    生成报告与退出码.
- 就地运行(worktree友好):
  - `autopilot run`必须在一个"已初始化Git仓库"内运行(或显式指定`--repo-dir`).
  - 启动前做硬性自检:
    - `git rev-parse --show-toplevel`必须成功.
    - 当配置使用`workspace.strategy=worktree`时,`git worktree`必须可用.
- 录制与证据落盘:
  - 强制启用`--record-session`,并把该JSONL作为"最终成果"(判定主证据源).
  - 同时落盘辅助证据(用于排障,但不作为唯一判定来源):
    - stdout/stderr(tee到文件).
    - `.ralph/current-events`指向的events文件路径与内容引用.
    - `.ralph/diagnostics/*`(当启用时).
- 自动判定(硬断言 + agent分析):
  - 硬断言: 从record-session JSONL中解析`bus.publish`事件序列.
    检查关键topic链路是否闭环,commit字段是否存在,以及是否出现禁止topic(如`gate.*`,`routing.escalate`等).
  - agent分析: 程序从JSONL抽取"可审计证据包"(topic统计,关键payload摘要,LOOP_COMPLETE上下文等).
    然后驱动一次轻量分析.
    输出结构化结论(是否满足程序设计要求,风险,建议).
  - 输出: 生成`report.md`与`report.json`.
    并用稳定退出码表达结果,便于无人值守调用.
- 新增本地skill(辅助使用与复盘):
  - 在本仓库`.codex/skills/`下新增一个skill.
  - 把"如何运行autopilot,如何读取report,失败时优先看哪些证据文件"固化为可复用流程,降低长期维护成本.

## Capabilities

### New Capabilities

- `autopilot-record-session-jsonl-verdict`: 在既有Git仓库内就地运行`ralph run --record-session ...`.
  并以录制JSONL为主证据源做自动判定与报告生成.
  支持无人值守调用.

### Modified Capabilities

(无)

## Impact

- 受影响代码区域:
  - `crates/ralph-cli`: 新增`autopilot`子命令与实现(运行封装,JSONL解析,报告生成,退出码语义).
  - `crates/ralph-core`: 可能需要复用/暴露record-session JSONL的解析能力(例如SessionPlayer/Record结构),以避免在CLI侧重复实现解析器.
- 受影响文档与资产:
  - 需要新增或更新使用文档.
    说明"无人值守autopilot"的约束(必须是Git repo,worktree语义,record-session路径).
  - 新增本地skill目录与脚本,作为长期可复用的操作指南.
