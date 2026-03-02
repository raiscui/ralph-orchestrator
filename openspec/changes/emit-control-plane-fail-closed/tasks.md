## 1. CLI fail-closed guards (`ralph emit`)

- [ ] 1.1 在 `crates/ralph-cli` 中,当检测到 `RALPH_HAT_INSTANCE_ID` 存在时,拒绝 `ralph emit --turn-action steer|interrupt` 并输出可行动的错误信息
- [ ] 1.2 在 `crates/ralph-cli` 中,当 `--turn-action steer|interrupt` 被使用时,强制要求 `--target-instance ralph#1`(缺失或非 ralph#1 均拒绝)
- [ ] 1.3 为以上两条新增 CLI 回归测试(至少覆盖: hat 环境拒绝,缺失 target_instance 拒绝,非 ralph#1 拒绝)

## 2. Supervisor fail-closed validation (external JSONL ingest)

- [ ] 2.1 在 `crates/ralph-core` 的外部事件 ingest 路径中,对 `turn_action=steer|interrupt` 做最终校验: 仅允许 `target_instance=ralph#1`,否则拒绝且不路由
- [ ] 2.2 拒绝时增加可观测性: 输出 warning(日志)并包含拒绝原因与关键字段(turn_action/target_instance/topic)
- [ ] 2.3 新增 `crates/ralph-core` 路由层回归测试: `turn_action` 事件不得被改投到 `ralph#2`(即使 `ralph#1` Running)
- [ ] 2.4 拒绝时必须给 `ralph#1` 发一个可见告警事件(优先复用 `routing.escalate`),避免无人值守时“看起来像没回应”

## 3. TUI 本地预检(减少黑盒排障)

- [ ] 3.1 在 `crates/ralph-tui` 中,限制 `!steer`/`!interrupt` 仅允许作用于 `ralph#1`;目标非 `ralph#1` 时直接报错且不写入外部事件 JSONL
- [ ] 3.2 为 TUI 侧限制新增最小回归测试(至少断言: 非 ralph#1 时不会调用 ExternalEventWriter 写入 turn_action)

## 4. Docs/spec 同步(面向 code agent)

- [ ] 4.1 更新 `specs/parallel-event-channels.spec.md`,补充 control-plane 边界: `turn_action=steer|interrupt` 仅 ExternalInput -> `ralph#1`
- [ ] 4.2 (可选) 更新 `examples/parallel-experimental-dev-engine/README.md`,明确 steer/interrupt 只能 target `ralph#1`,并提示 hats 不得使用 `--turn-action`
- [ ] 4.3 更新 `specs/parallel-event-channels.spec.md`,补充 hat-to-hat 的 request/result 约定: B hat 只在 job/turn 结束时回传最终结论,不在中途 reply
- [ ] 4.4 更新 `config/all_hat.md`,移除/改写会误导 hats 的示例(例如对 `writer#1` steer),改为仅对 `ralph#1` steer/interrupt,并明确 hats 禁止使用 `--turn-action`

## 5. 验证与回归

- [ ] 5.1 运行 `cargo fmt --check`
- [ ] 5.2 运行最小测试集: `cargo test -p ralph-cli` + `cargo test -p ralph-core`
- [ ] 5.3 运行 smoke fixtures: `cargo test -p ralph-core smoke_runner`
