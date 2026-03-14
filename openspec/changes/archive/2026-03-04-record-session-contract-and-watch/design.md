## Context

现状里,record-session 已经具备"可回放 JSONL"的基本形态,并且已经开始承载更严肃的职责:

- 作为 replay fixture 的事实来源.
- 作为 autopilot hard verdict 的主证据源.
- 作为人工排障时的唯一可共享证据文件.

但我们缺少两块关键能力:

1) contract tests 没有覆盖 Ctrl+C/SIGTERM 这类可控中断路径.
2) 没有一个轻量工具把 record-session 快速转成"可扫一眼的状态摘要".

这会导致:

- 大量 E2E 测试也可能全绿.
- 真实使用时,用户仍会拿到一份"信息不全"的 JSONL,并产生误判.

## Goals / Non-Goals

Goals:

- 用低成本,高确定性的方式,把 record-session 的 durability/parseability 变成硬门槛.
- 提供 `ralph record summary/watch` 作为面向人类的排障入口.
- 解析逻辑复用,避免 autopilot 与 record 工具各写一套而漂移.

Non-Goals:

- 不追求 SIGKILL/断电级别的强保证(那需要更重的 fsync 契约).
- 不做 TUI 化,不做复杂交互(先把"信息密度"与"稳定性"做好).
- 不改变 `ralph run` 的业务语义,record 工具只读 JSONL.

## Design Sketch

数据流可以抽象成两条线:

```
ralph run
  -> record-session JSONL (append-only)
     -> replay/autopilot (strict parse, post-run)
     -> record watch (raw follow, during-run)
```

### 1) `record summary` vs `record watch`

- `record summary` 面向"已完成的 JSONL":
  - 可以严格解析,遇到 JSONL 不合法直接报错退出.
  - 适合 CI 脚本,也适合人类快速扫摘要.
- `record watch` 面向"增长中的 JSONL":
  - 默认以 raw 方式输出新增的完整 JSONL 行(像 `tail -f`).
  - 必须容忍末尾半行 JSON(没有换行符的 incomplete line),不得将其当作 malformed.
  - 必须可持续运行,不要因为一次 parse error 或一次 I/O 抖动就退出.

### 2) watch 的两种实现路线

方向 1: 全量重读 + 找新增行(先能用,实现更快)

- 每次 tick 都从头读到尾,用行数或字节数找“新增长的完整行”,并输出.
- 优点: 代码短,正确性直观.
- 代价: 文件大时 O(n) 重算,长跑时会有明显 CPU/IO.

方向 2: 增量读取 + partial line buffer(最佳方案,长期更稳)

- 维护 file offset + partial line buffer.
- 只对新增的完整行(以 `\n` 结尾)做输出.
- 优点: 复杂度可控,且性能与文件大小解耦.
- 代价: 实现略复杂,需要单测覆盖 partial line 与 offset 语义.

本 change 倾向选择方向 2.
原因是 record-session 作为证据流可能很大,watch 的预期使用场景又是"长时间跟随".

### 3) 输出形态(默认行为)

本 change 里,`summary` 与 `watch` 的默认输出策略刻意分工:

- `ralph record summary <FILE>`: 默认输出人类可读摘要(用于“扫一眼状态”).
- `ralph record watch [FILE]`: 默认输出 raw JSONL 行(用于“确认证据在增长/抓取原始证据”).

其中,summary 的默认摘要块建议固定为 4 个块,每块尽量短:

1) Meta: cwd, workspace_root, argv_joined, pid.
2) Termination: reason(若存在),以及最后一条 record 的时间戳.
3) Topics: `bus.publish` 的 topic top N,以及最近 M 个 topic timeline(可选).
4) Stdout tail: `ux.terminal.write(stdout=true)` 的尾部窗口(按 chunk 或按行).

watch 的默认输出则尽量“零抽象”:

- 只要出现新增的完整行,就原样写到 stdout.
- 末尾 incomplete line 不输出,等待下一次写入补齐换行符.
- 不强制解析 JSON,避免把“看证据是否在增长”变成“受 schema 漂移影响的工具”.

### 4) watch 的自动定位: `.ralph/record-session.latest`

你已确认 `.ralph/` 是需要保留的证据(不是随时可删的临时物),但 record-session 文件不一定放在 `.ralph/`.
因此 watch 的无参自动定位不应依赖“扫描磁盘猜路径”,而应依赖显式证据指针:

- 当 `ralph run --record-session <FILE>` 启用时:
  - 在 workspace_root 写入 `.ralph/record-session.latest`,内容为 `<FILE>` 的绝对路径(或可解析为绝对路径).
- 当 `ralph record watch` 不传 `FILE` 时:
  - 从 cwd 向上定位 workspace_root.
  - 读取 `.ralph/record-session.latest` 并 follow 该文件.

### 5) 可控中断语义: 证据完整优先(写 termination 再退出)

本 change 里,Ctrl+C/SIGTERM 的 contract 目标是:

- 尽量写入 `_meta.termination(reason=Interrupted)` 并 flush.
- JSONL 保持“逐行可解析”(complete line 视为有效证据).
- 再做退出与子进程清理.

这意味着中断实现不应采用“kill 自己所在进程组”的一刀切做法.
更可靠的路线是:后端子进程各自成为新的进程组,中断时只杀对应子进程组,而 orchestrator 自身先完成落盘收尾.

### 6) 并行模式下 iteration 的定义(用于 termination 元信息)

并行模式下,`iteration` 不应试图复刻串行的“一轮对话”语义.
本 change 建议采用与并行硬护栏一致的口径:

- parallel iteration = 协调者 `ralph` 的 job 完成次数.
- `_meta.termination.data.iterations` 在 parallel 下应写入该值(而不是占位 0),用于排障时粗略判断“跑了几轮收敛”.

### 7) 解析逻辑复用策略

`autopilot` 现在已经有一套 record-session 的 strict parse + 聚合能力.
为了避免未来 autopilot 与 record 工具对"topic_counts / terminal_tail"的口径漂移,建议:

- 抽离一个共享模块,输出"通用统计摘要".
- autopilot 在此摘要之上再叠加自己的 hard verdict 断言逻辑.

这样 record 工具也不会把 autopilot 的"必需 topic/禁止 topic"这种业务规则带进来.

## Test Strategy (Backpressure)

本 change 的关键不是再加更多 E2E.
关键是补齐两类专用测试:

1) unit:
   - watch: 覆盖 partial line 容忍,offset 增量语义,以及“只输出完整行”的不变量.
   - summary/autopilot: 覆盖 strict parse 与聚合口径(避免未来漂移).
2) integration: 覆盖 Ctrl+C/SIGTERM 的可控中断:
   - 启动 `ralph run`(使用 parallel `--idle-start`,避免依赖真实 backend).
   - 等待 JSONL 出现 `_meta.session_start`.
   - 对子进程发送 SIGINT.
   - 断言 JSONL 逐行可解析,并包含 `_meta.session_start/_meta.loop_start/_meta.termination(reason=Interrupted)`.

这条 integration test 会成为未来的 guardrail.
任何人把 flush 或 SIGINT 收尾弄坏,CI 会立刻红,而不是等用户线上踩坑.
