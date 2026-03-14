## 1. Delta Spec

- [x] 1.1 新增 delta spec,固化 record-session contract 与 `ralph record` 命令行为
- [x] 1.2 明确 scope: 只覆盖 SIGINT/SIGTERM(可控中断),不承诺 SIGKILL/断电级别

## 2. Shared Parser Module

- [x] 2.1 抽离一份通用的 record-session strict 解析与聚合模块(面向 summary/autopilot 复用)
- [x] 2.2 autopilot 迁移到共享模块(行为不变,仅减少重复与漂移)

## 3. `ralph record summary`

- [x] 3.1 在 `ralph --help` 中可发现 `record` 命令族
- [x] 3.2 实现 `ralph record summary <FILE>`:
  - 输出 meta(session_start/loop_start),termination(reason),topic_counts(top N),stdout tail
  - 对已完成 JSONL 走 strict parse,发现非法 JSONL 要给出可读错误

## 4. `ralph record watch`

- [x] 4.1 实现 `ralph record watch [FILE]`(默认 raw follow):
  - follow 增长中的 JSONL,像 `tail -f` 一样输出新增的完整 JSONL 行(不重排/不改写)
  - 容忍末尾 incomplete line(没有 `\\n`),不得将其当 malformed 或提前输出
  - 运行中遇到一次读/解析异常不得直接退出(写 stderr 提示并继续)
- [x] 4.2 自动定位:
  - 当未提供 `FILE` 时,从 workspace_root 的 `.ralph/record-session.latest` 解析最近一次录制路径
  - 启用 `--record-session <FILE>` 时,必须写入该指针文件(内容可解析为绝对路径)
- [x] 4.3 参数(最小集):
  - `--interval-ms`(poll 间隔,默认 200-500ms 即可)
  - `--from-start`(可选: 从文件开头输出; 默认从当前 EOF 开始 follow)

## 5. Tests (核心 backpressure)

- [x] 5.1 unit:
  - watch: 覆盖 partial line 语义,以及“只输出完整行”的不变量
  - summary: 覆盖 strict parse 与聚合口径(与 autopilot 共享)
- [x] 5.2 integration: Ctrl+C/SIGINT 终止路径回归测试:
  - 启动 `ralph run --idle-start --record-session ... --no-tui`(parallel 模式)
  - 等待 `_meta.session_start` 落盘
  - 发送 SIGINT
  - 断言 JSONL 逐行可解析,并包含 termination(reason=Interrupted)
- [x] 5.3 integration: record-session.latest 指针回归测试:
  - 启动 `ralph run --record-session ...`
  - 断言 `.ralph/record-session.latest` 写入且内容可解析
  - `ralph record watch` 无参可定位到该路径

## 6. Docs / Examples

- [x] 6.1 在相关 docs/specs 中补充 `ralph record summary/watch` 的使用示例
- [x] 6.2 增加一段"为什么 E2E 可能抓不到该类问题,以及我们如何用 contract tests 兜底"的说明
