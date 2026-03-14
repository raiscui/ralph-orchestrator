## Context

现状:

- `ralph run --record-session <file>.jsonl`已经能把关键执行过程录制为JSONL:
  - `bus.publish`: 事件总线的业务事件(topic+payload等).
  - `ux.terminal.write`: 终端输出(用于复现stdout视角).
  - `_meta.*`: loop_start/iteration/termination等元信息(用于回放与诊断).
- 该JSONL是可回放的"统一证据源",也是你强调的"最终成果".
- 但当前缺少一条"无人干预"的标准化流程,把:
  - 运行(就地Git仓库,支持`git worktree`)
  - 录制(落盘JSONL+辅助证据)
  - 分析(硬断言+agent判定)
  - 判定(稳定退出码+报告)
 组合成一条可重复执行的机制.

约束:

- 目标目录(例如`parallel-experimental-dev-engine`)本身是已初始化的Git仓库.
- 并行runner依赖`git worktree`隔离job,因此不能通过"复制到临时workspace"来跑.
- 你不希望跑既有`ralph-e2e` harness,而是直接跑真实的`ralph run --record-session ...`.
- 无人值守意味着:
  - 不能依赖人工`gate.resolve`.
  - 任何需要人工介入的信号(例如`gate.request`)都必须被自动判为失败并给出可审计证据.

## Goals / Non-Goals

**Goals:**

- 新增`ralph autopilot`命令族,把"就地运行 + 录制 + 分析 + 判定"固化成可无人值守的一条命令.
- 把`--record-session`生成的JSONL作为主证据源:
  - 硬断言必须只依赖JSONL(不依赖stdout是否被截断).
  - 报告必须引用JSONL中可定位的证据(例如按record索引或时间戳定位).
- 提供两种模式:
  - `autopilot run`: 执行一次`ralph run --record-session ...`并在结束后分析JSONL.
  - `autopilot analyze`: 只分析既有JSONL,不重新运行.
- 分析分两层:
  - 硬断言(确定性): topic闭环,禁止topic,commit字段,termination reason等.
  - agent分析(启发式/质量): 基于程序抽取的"证据包"给出结构化结论与风险建议.
- 输出机器可用报告:
  - `report.json`(退出码/断言结果/agent分析结果/证据路径).
  - `report.md`(人类可读).
- 增加本地skill,把运行与复盘路径固化为可复用流程.

**Non-Goals:**

- 不改变既有`ralph run`的完成语义与并行协议(autopilot只是包装与判定层).
- 不把`ralph-e2e`替换为autopilot(两者定位不同;autopilot面向"真实仓库就地跑").
- 不在本change里实现"交互式恢复/自动补gate"之类行为(无人值守场景直接判失败,并输出证据).
- 不录制TUI逐帧画面(本change以JSONL为最终成果与证据源).

## Decisions

### 1) 落点: 在`crates/ralph-cli`新增`autopilot`子命令

选择: 把autopilot实现为`ralph`二进制的新子命令,而不是单独的脚本或新crate.

理由:

- 你要求"程序配合",并希望一条命令可无人值守运行.
- 录制JSONL与回放/解析能力已在`ralph-core`存在,CLI最适合做薄封装与退出码语义.

备选(未选):

- 仅写脚本: 可用但容易漂移,且难以共享解析器与退出码语义.
- 新二进制: 需要额外分发与版本管理,不如直接扩展`ralph`.

### 2) `autopilot run`的执行模型: 子进程运行真实`ralph run`

选择:

- `autopilot run`通过`std::process::Command`启动一个子进程执行`ralph run`.
- 子进程的`current_dir`必须设置为`--repo-dir`(默认`.`),确保:
  - `git worktree`在正确的repo中执行.
  - `.ralph/*`等相对路径落在目标repo中.

理由:

- 你明确要求跑"真实的`ralph run --record-session`",而不是测试harness的模拟行为.
- 子进程方式可以保证与用户手工运行时的行为一致,并避免在同一进程内重入event loop带来的复杂性.

### 3) 主证据源: 以record-session JSONL为唯一判定输入

选择:

- 硬断言只读取`--record-session`生成的JSONL,通过解析其中的`bus.publish`记录来判断topic链路与关键字段.
- stdout/stderr与`.ralph/events*.jsonl`只作为"辅助证据"落盘,用于排障与人类复核,不作为唯一判定来源.

理由:

- 你强调JSONL是最终成果,应当可独立复盘与自动判定.
- stdout可能截断或受渲染影响,而`bus.publish`是结构化信号,更稳.

### 4) JSONL解析器: 复用`ralph-core`的SessionPlayer/Record模型

选择:

- 在CLI侧复用`ralph-core`提供的record-session解析能力(不要再写一套JSONL解析).
- 只抽取需要的最小视图:
  - `bus.publish`的`Event`列表(用于topic/commit/禁止topic判定).
  - `_meta.termination`(用于completion判定).
  - `ux.terminal.write`的尾部窗口(用于定位`LOOP_COMPLETE`上下文,写入报告).

理由:

- 复用能减少协议漂移风险.
- 保持与回放fixture格式一致,便于未来把autopilot产物复用为fixture.

### 5) 硬断言集合: "严格闭环 + 禁止人工介入信号"

选择: 默认硬断言至少包含:

- 必须出现的topic:
  - `experiment.task`
  - `experiment.result`(payload必须包含`commit`)
  - `experiment.reviewed`(payload必须包含`evidence_ok=true`)
  - `integration.task`
  - `integration.applied`
  - `experiment.complete`
- 禁止出现的topic(出现即失败):
  - `gate.request` / `gate.timeout` / `gate.resolve`
  - `routing.escalate`
- 终止原因必须为`CompletionPromise`(来自`_meta.termination.reason`)

理由:

- 这套规则直接对应"无人值守闭环"的最小合同.
- `gate.*`出现意味着需要人工介入,与无人值守目标冲突.

### 6) agent分析: 用"证据包"驱动一次轻量分析,输出结构化JSON

选择:

- 程序从record-session JSONL中抽取并生成一个"证据包"(JSON),包含:
  - topic计数与出现顺序摘要
  - 每个experiment的commit与关键证据摘要
  - termination reason与`LOOP_COMPLETE`附近输出窗口
  - 命中的禁止topic列表(若有)
- 然后再启动一次轻量分析(同样以子进程`ralph run`执行),使用一个内建的极小`ralph.yml`:
  - 只有一个analyzer hat,输入为证据包+设计要求
  - 输出必须为`<event topic="analyze.complete">{...json...}</event>`并以`ANALYSIS_COMPLETE`结束
- CLI解析`analyze.complete`里的JSON,写入`report.json`.

理由:

- 你要求"agent分析最终JSONL是否达到程序设计要求".
- 先抽取证据包可以控制输入体积,避免把整份JSONL塞给模型导致不可控成本与截断风险.

备选(未选):

- 直接用正则/规则判定质量: 会把"设计要求是否满足"过度简化,难以覆盖真实漂移案例.

### 7) 退出码语义: 为无人值守提供稳定信号

选择:

- `0`: 硬断言通过,且agent分析通过(或显式`--skip-agent-analysis`).
- `1`: 硬断言失败(闭环未达成/出现禁止topic/非CompletionPromise等).
- `2`: 硬断言通过,但agent分析判定不达标(或质量分过低).
- `3`: 需要agent分析但分析过程失败(超时/解析失败/后端不可用等).

理由:

- 无人值守场景需要退出码作为唯一信号,无需人工读取stdout.

## Risks / Trade-offs

- [Risk] record-session JSONL体积过大导致读取/分析成本高
  - Mitigation: 证据包抽取做尺寸预算(只保留尾部窗口与必要字段),报告引用按索引定位.
- [Risk] 子进程运行`ralph run`可能遗留worktree(崩溃/强杀)
  - Mitigation: autopilot在启动前提供可选的"清理`.ralph/worktrees`残留"模式,并在报告中提示残留路径.
- [Risk] 不同后端输出漂移导致stdout判断不稳定
  - Mitigation: topic判定只依赖`bus.publish`;stdout只用于辅助上下文,不作为硬门槛.
- [Risk] agent分析本身不确定
  - Mitigation: 输出必须结构化JSON,并把硬断言结果与证据包一并落盘,便于复盘与迭代提示词.

## Migration Plan

1. 新增`ralph autopilot`子命令与对应文档/skill.
2. 不改动既有命令行为,不需要迁移旧数据.
3. 通过回归测试锁定:
   - record-session JSONL解析与硬断言逻辑
   - agent分析输出格式解析
   - 退出码语义

## Open Questions

- 是否需要支持"自定义必需/禁止topic集合"(作为可选参数),还是先固化为严格闭环的默认集合.
- agent分析使用的backend是否需要默认跟随主run的backend(推荐),以及是否允许单独覆盖.
