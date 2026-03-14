## 1. CLI Surface

- [x] 1.1 在`crates/ralph-cli`新增`autopilot`子命令,并提供`run`与`analyze`两个子命令入口
- [x] 1.2 定义并实现CLI参数(至少包含): `--repo-dir`,`--config`,`--record-session`,`--out-dir`,`--skip-agent-analysis`,`--analysis-backend`(或等价能力)
- [x] 1.3 `ralph autopilot --help`与子命令帮助信息可发现,并明确"必须在Git仓库内就地运行"的约束

## 2. `autopilot run`: 就地运行封装(真实`ralph run`)

- [x] 2.1 实现`--repo-dir`的Git仓库自检(`git rev-parse --show-toplevel`)与`git worktree`可用性检查(在启动子进程前失败)
- [x] 2.2 实现`--record-session`目标路径可写检查(在启动子进程前失败)
- [x] 2.3 以子进程方式启动真实的`ralph run`:
  - current_dir必须为`--repo-dir`
  - 必须强制传入`--no-tui`
  - 必须强制传入`--record-session <file>.jsonl`
- [x] 2.4 将子进程stdout/stderr tee落盘到`<out-dir>/stdout.txt`与`<out-dir>/stderr.txt`(作为辅助证据)

## 3. record-session JSONL 解析与硬断言

- [x] 3.1 复用`ralph-core`的record-session结构(Record/SessionPlayer)实现JSONL读取,并能抽取:
  - `bus.publish`事件列表(topic+payload)
  - `_meta.termination`中的`reason`
  - `ux.terminal.write`的尾部窗口(用于报告上下文)
- [x] 3.2 实现硬断言判定:
  - 必须出现: `experiment.task`,`experiment.result`,`experiment.reviewed`,`integration.task`,`integration.applied`,`experiment.complete`
  - `experiment.result` payload必须包含`commit`
  - `experiment.reviewed` payload必须包含`evidence_ok=true`(允许YAML或JSON口径)
  - 禁止出现: `gate.request`,`gate.resolve`,`gate.timeout`,`routing.escalate`
  - `_meta.termination.reason`必须为`CompletionPromise`
- [x] 3.3 为硬断言输出结构化结果(每条断言包含: passed/expected/actual/evidence_refs),供report.json消费

## 4. 证据包与agent分析

- [x] 4.1 从record-session JSONL生成`analysis_input.json`(证据包),并实现体积预算(避免无限增长)
- [x] 4.2 实现agent分析执行:
  - 生成最小分析用`ralph.yml`与prompt文件
  - 以子进程运行`ralph run --no-tui`做一次轻量分析
  - 解析输出中的`<event topic="analyze.complete">...JSON...</event>`为`analysis_output.json`
- [x] 4.3 实现退出码语义:
  - 0: 硬断言PASS且agent verdict PASS(或显式跳过agent分析)
  - 1: 硬断言FAIL
  - 2: 硬断言PASS但agent verdict FAIL或`quality_score=suboptimal`
  - 3: 需要agent分析但分析运行/解析失败

## 5. 报告与产物布局

- [x] 5.1 实现`report.json`写入(机器可读),包含:
  - analyzed/run的record-session绝对路径或可解析路径
  - 硬断言结果
  - agent分析摘要(或跳过原因)
  - 最终退出码与原因
- [x] 5.2 实现`report.md`写入(人类可读),并列出失败时优先查看的证据文件路径
- [x] 5.3 固化`<out-dir>`目录结构(至少包含): `stdout.txt`,`stderr.txt`,`analysis_input.json`,`analysis_output.json`,`report.json`,`report.md`

## 6. Tests

- [x] 6.1 增加最小record-session JSONL fixture(至少PASS与FAIL各一份),用于稳定回归测试
- [x] 6.2 增加单元测试覆盖:
  - JSONL解析与事件抽取
  - 硬断言判定(必需topic/禁止topic/commit/evidence_ok/termination)
  - 退出码映射逻辑
- [x] 6.3 增加报告schema测试(至少断言`report.json`包含关键字段)

## 7. Local Skill

- [x] 7.1 新增`.codex/skills/parallel-engine-autopilot/SKILL.md`,写清:
  - 何时触发该skill(典型用户说法)
  - 如何运行`ralph autopilot run`与`ralph autopilot analyze`
  - 失败时如何按证据文件快速定位
- [x] 7.2 新增skill脚本:
  - `scripts/run_autopilot.sh`(封装一条标准运行命令)
  - `scripts/summarize_report.py`(读取`report.json`输出短摘要)

## 8. Docs(可选,但推荐)

- [x] 8.1 在README或docs中补充autopilot的定位与使用示例(强调"就地Git仓库 + record-session JSONL为最终成果")
